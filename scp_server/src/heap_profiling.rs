//! Opt-in heap profiles. No socket or background task exists without a listen address.
#[cfg(target_os = "linux")]
use std::time::Duration;
use std::{future::Future, io};
#[cfg(target_os = "linux")]
use tokio::time::timeout;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[cfg(target_os = "linux")]
pub async fn start_from_env() -> io::Result<()> {
    let addr = std::env::var_os("SCPD_PPROF_LISTEN");
    let enabled = std::env::var_os("GROUNDHOG_HEAP_PROFILING")
        .is_some_and(|value| value == "1");
    // Do not even initialize the profiler controller when profiling is disabled.
    if enabled {
        let ctl = jemalloc_pprof::PROF_CTL.as_ref().ok_or_else(|| {
            io::Error::other("jemalloc profiling is unavailable")
        })?;
        ctl.lock().await.activate().map_err(io::Error::other)?;
    }
    let addr = addr
        .map(|value| {
            value.into_string().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "invalid SCPD_PPROF_LISTEN",
                )
            })
        })
        .transpose()?;
    let Some(listener) = bind_listener(addr.as_deref()).await? else {
        return Ok(());
    };
    tracing::info!(addr = %listener.local_addr()?, "heap profile listener enabled");
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(async move {
                        // Bound slow/incomplete HTTP clients, including writes.
                        let _ = timeout(
                            Duration::from_secs(10),
                            serve_connection(stream, enabled, dump_heap),
                        )
                        .await;
                    });
                }
                Err(err) => {
                    tracing::error!(%err, "heap profile listener failed");
                    break;
                }
            }
        }
    });
    Ok(())
}

#[cfg(target_os = "linux")]
async fn dump_heap() -> io::Result<Vec<u8>> {
    // Dumping and symbolizing are blocking; keep them off Tokio's async workers.
    tokio::task::spawn_blocking(|| {
        let ctl = jemalloc_pprof::PROF_CTL.as_ref().ok_or_else(|| {
            io::Error::other("jemalloc profiling unavailable")
        })?;
        let mut ctl = ctl.blocking_lock();
        ctl.dump_pprof().map_err(io::Error::other)
    })
    .await
    .map_err(io::Error::other)?
}

async fn bind_listener(addr: Option<&str>) -> io::Result<Option<TcpListener>> {
    match addr {
        Some(addr) => TcpListener::bind(addr).await.map(Some),
        None => Ok(None),
    }
}

async fn serve_connection<F, Fut>(
    mut stream: TcpStream,
    enabled: bool,
    dump: F,
) -> io::Result<()>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = io::Result<Vec<u8>>>,
{
    // This listener supports one request per connection, no request bodies.
    let mut request = Vec::new();
    let mut chunk = [0; 1024];
    let valid = loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(());
        }
        request.extend_from_slice(&chunk[..n]);
        if request.len() > 8192 {
            break false;
        }
        if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            break true;
        }
    };
    let line = request
        .split(|byte| *byte == b'\r')
        .next()
        .unwrap_or_default();
    let route = line == b"GET /debug/pprof/heap HTTP/1.1"
        || line == b"GET /debug/pprof/heap HTTP/1.0";
    let (status, body) = if !valid {
        ("400 Bad Request", Vec::new())
    } else if !enabled || !route {
        ("404 Not Found", Vec::new())
    } else {
        match dump().await {
            Ok(body) => ("200 OK", body),
            Err(err) => {
                tracing::warn!(%err, "heap profile dump failed");
                ("500 Internal Server Error", Vec::new())
            }
        }
    };
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(&body).await?;
    stream.shutdown().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unset_listen_address_creates_no_listener() {
        assert!(bind_listener(None).await.unwrap().is_none());
    }

    async fn request(enabled: bool, line: &str, fail: bool) -> Vec<u8> {
        let listener =
            bind_listener(Some("127.0.0.1:0")).await.unwrap().unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            serve_connection(stream, enabled, || async move {
                assert!(enabled, "disabled route must never dump");
                if fail {
                    Err(io::Error::other("test dump failure"))
                } else {
                    Ok(b"profile protobuf".to_vec())
                }
            })
            .await
            .unwrap();
        });
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(format!("{line}\r\nHost: localhost\r\n\r\n").as_bytes())
            .await
            .unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        task.await.unwrap();
        response
    }

    #[tokio::test]
    async fn disabled_heap_endpoint_is_404() {
        let response =
            request(false, "GET /debug/pprof/heap HTTP/1.1", false).await;
        assert!(response.starts_with(b"HTTP/1.1 404 Not Found\r\n"));
    }

    #[tokio::test]
    async fn enabled_heap_endpoint_serves_profile() {
        let response =
            request(true, "GET /debug/pprof/heap HTTP/1.1", false).await;
        assert!(response.starts_with(b"HTTP/1.1 200 OK\r\n"));
        assert!(response.ends_with(b"profile protobuf"));
        assert!(
            String::from_utf8_lossy(&response)
                .contains("Content-Length: 16\r\n")
        );
    }

    #[tokio::test]
    async fn other_routes_and_methods_are_404() {
        for line in [
            "GET /debug/pprof/heap/ HTTP/1.1",
            "GET /debug/pprof/profile HTTP/1.1",
            "POST /debug/pprof/heap HTTP/1.1",
        ] {
            // A failing dump would yield 500 if the route incorrectly matched.
            assert!(
                request(true, line, true)
                    .await
                    .starts_with(b"HTTP/1.1 404 Not Found\r\n")
            );
        }
    }

    #[tokio::test]
    async fn dump_failure_is_500() {
        assert!(
            request(true, "GET /debug/pprof/heap HTTP/1.1", true)
                .await
                .starts_with(b"HTTP/1.1 500 Internal Server Error\r\n")
        );
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn jemalloc_starts_inactive_and_can_dump_pprof() {
        let ctl = jemalloc_pprof::PROF_CTL.as_ref().unwrap();
        {
            let mut ctl = ctl.lock().await;
            assert!(!ctl.activated());
            assert_eq!(ctl.lg_sample(), 19);
            ctl.activate().unwrap();
            assert!(ctl.activated());
        }
        let allocation = vec![42u8; 4 * 1024 * 1024];
        std::hint::black_box(&allocation);
        let profile = dump_heap().await;
        // Restore shared allocator state even if the conversion failed.
        ctl.lock().await.deactivate().unwrap();
        let profile = profile.unwrap();
        assert!(profile.starts_with(&[0x1f, 0x8b]), "expected gzipped pprof");
    }
}
