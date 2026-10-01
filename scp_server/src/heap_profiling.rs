//! Opt-in heap profiles. No socket or background task exists without a listen address.
use std::time::Duration;
use std::{future::Future, io};
use tokio::time::timeout;
static DUMP_SLOT: std::sync::LazyLock<std::sync::Arc<tokio::sync::Semaphore>> =
    std::sync::LazyLock::new(|| {
        std::sync::Arc::new(tokio::sync::Semaphore::new(1))
    });
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[cfg(target_os = "linux")]
pub async fn start_from_env() -> io::Result<()> {
    let addr = std::env::var_os("SCPD_PPROF_LISTEN");
    let enabled = std::env::var_os("GROUNDHOG_HEAP_PROFILING")
        .is_some_and(|value| value == "1");
    // Leave allocator profiling controls untouched when sampling is disabled.
    if enabled {
        if !read_bool(c"config.prof")? || !read_bool(c"opt.prof")? {
            return Err(io::Error::other(
                "jemalloc profiling unavailable: config.prof and opt.prof must be true",
            ));
        }
        set_active(true)?;
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
                        let _ =
                            serve_connection(stream, enabled, dump_heap).await;
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
async fn dump_heap(
    permit: tokio::sync::OwnedSemaphorePermit,
) -> io::Result<Vec<u8>> {
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        use std::{
            ffi::CString,
            os::unix::{ffi::OsStrExt, fs::OpenOptionsExt},
        };
        // /dev/urandom supplies an unpredictable suffix; create_new is O_EXCL.
        let mut random = [0u8; 16];
        std::io::Read::read_exact(
            &mut std::fs::File::open("/dev/urandom")?,
            &mut random,
        )?;
        let suffix: String =
            random.iter().map(|b| format!("{b:02x}")).collect();
        let path = std::env::temp_dir().join(format!("scpd-heap-{suffix}"));
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let cleanup = Cleanup(path);
        let filename = CString::new(cleanup.0.as_os_str().as_bytes())
            .map_err(io::Error::other)?;
        let mut pointer = filename.as_ptr();
        // SAFETY: prof.dump expects a pointer to a live const char* filename.
        let result = unsafe {
            tikv_jemalloc_sys::mallctl(
                c"prof.dump".as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                (&mut pointer as *mut *const std::ffi::c_char).cast(),
                std::mem::size_of_val(&pointer),
            )
        };
        ctl_result(result)?;
        drop(file);
        crate::heap_profile::jemalloc_to_pprof(
            &std::fs::read_to_string(&cleanup.0)?,
            &std::fs::read_to_string("/proc/self/maps")?,
        )
    })
    .await
    .map_err(io::Error::other)?
}

#[cfg(target_os = "linux")]
fn ctl_result(result: i32) -> io::Result<()> {
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "jemalloc mallctl failed: {}",
            io::Error::from_raw_os_error(result)
        )))
    }
}
#[cfg(target_os = "linux")]
fn read_bool(name: &std::ffi::CStr) -> io::Result<bool> {
    let mut value = false;
    let mut len = std::mem::size_of_val(&value);
    // SAFETY: named bool control writes into a correctly sized bool.
    ctl_result(unsafe {
        tikv_jemalloc_sys::mallctl(
            name.as_ptr(),
            (&mut value as *mut bool).cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    })
    .map_err(|err| {
        io::Error::other(format!(
            "jemalloc profiling unavailable: cannot read {}: {err}",
            name.to_string_lossy()
        ))
    })?;
    Ok(value)
}
#[cfg(target_os = "linux")]
fn set_active(mut value: bool) -> io::Result<()> {
    // SAFETY: prof.active accepts a bool with this size.
    ctl_result(unsafe {
        tikv_jemalloc_sys::mallctl(
            c"prof.active".as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            (&mut value as *mut bool).cast(),
            std::mem::size_of_val(&value),
        )
    })
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
    F: FnOnce(tokio::sync::OwnedSemaphorePermit) -> Fut,
    Fut: Future<Output = io::Result<Vec<u8>>>,
{
    // This listener supports one request per connection, no request bodies.
    let mut request = Vec::new();
    let mut chunk = [0; 1024];
    let valid = timeout(Duration::from_secs(10), async {
        loop {
            let n = stream.read(&mut chunk).await?;
            if n == 0 {
                return Ok::<bool, io::Error>(false);
            }
            request.extend_from_slice(&chunk[..n]);
            if request.len() > 8192 {
                break Ok(false);
            }
            if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                break Ok(true);
            }
        }
    })
    .await
    .map_err(|_| {
        io::Error::new(io::ErrorKind::TimedOut, "request read timeout")
    })??;
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
        match DUMP_SLOT.clone().try_acquire_owned() {
            Err(_) => ("429 Too Many Requests", Vec::new()),
            Ok(permit) => {
                let result = dump(permit).await;
                match result {
                    Ok(body) => ("200 OK", body),
                    Err(err) => {
                        tracing::warn!(%err, "heap profile dump failed");
                        ("500 Internal Server Error", Vec::new())
                    }
                }
            }
        }
    };
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    timeout(Duration::from_secs(10), async {
        stream.write_all(header.as_bytes()).await?;
        stream.write_all(&body).await?;
        stream.shutdown().await
    })
    .await
    .map_err(|_| {
        io::Error::new(io::ErrorKind::TimedOut, "response write timeout")
    })?
}

#[cfg(test)]
mod tests {
    use super::*;
    static TEST_LOCK: tokio::sync::Mutex<()> =
        tokio::sync::Mutex::const_new(());

    #[tokio::test]
    async fn unset_listen_address_creates_no_listener() {
        assert!(bind_listener(None).await.unwrap().is_none());
    }

    async fn request(enabled: bool, line: &str, fail: bool) -> Vec<u8> {
        let _guard = TEST_LOCK.lock().await;
        let listener =
            bind_listener(Some("127.0.0.1:0")).await.unwrap().unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            serve_connection(stream, enabled, |permit| async move {
                let _permit = permit;
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

    #[tokio::test]
    async fn concurrent_request_is_429_and_long_dump_survives() {
        let _guard = TEST_LOCK.lock().await;
        let listener =
            bind_listener(Some("127.0.0.1:0")).await.unwrap().unwrap();
        let addr = listener.local_addr().unwrap();
        let (started, ready) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let first = tokio::spawn(serve_connection(
                stream,
                true,
                |permit| async move {
                    let _permit = permit;
                    started.send(()).unwrap();
                    tokio::time::sleep(Duration::from_secs(11)).await;
                    Ok(b"long profile".to_vec())
                },
            ));
            let (stream, _) = listener.accept().await.unwrap();
            serve_connection(stream, true, |_| async {
                panic!("busy request must not dump")
            })
            .await
            .unwrap();
            first.await.unwrap().unwrap();
        });
        let mut first = TcpStream::connect(addr).await.unwrap();
        let request = b"GET /debug/pprof/heap HTTP/1.1\r\n\r\n";
        first.write_all(request).await.unwrap();
        ready.await.unwrap();
        let mut second = TcpStream::connect(addr).await.unwrap();
        second.write_all(request).await.unwrap();
        let mut response = Vec::new();
        timeout(Duration::from_secs(1), second.read_to_end(&mut response))
            .await
            .unwrap()
            .unwrap();
        assert!(response.starts_with(b"HTTP/1.1 429"));
        response.clear();
        first.read_to_end(&mut response).await.unwrap();
        assert!(response.starts_with(b"HTTP/1.1 200"));
        assert!(response.ends_with(b"long profile"));
        task.await.unwrap();
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn jemalloc_starts_inactive_and_can_dump_pprof() {
        let _guard = TEST_LOCK.lock().await;
        assert!(!read_bool(c"prof.active").unwrap());
        set_active(true).unwrap();
        let allocation = vec![42u8; 4 * 1024 * 1024];
        std::hint::black_box(&allocation);
        let profile =
            dump_heap(DUMP_SLOT.clone().acquire_owned().await.unwrap()).await;
        // Restore shared allocator state even if the conversion failed.
        set_active(false).unwrap();
        let profile = profile.unwrap();
        assert!(!profile.is_empty());
    }
}
