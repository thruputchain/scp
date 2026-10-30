//! Opt-in heap profiles. No socket or background task exists without a listen address.
use std::net::{IpAddr, SocketAddr};
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

pub async fn start_from_env() -> io::Result<()> {
    let addr = std::env::var_os("SCPD_PPROF_LISTEN");
    let allow_remote = std::env::var_os("SCPD_PPROF_ALLOW_REMOTE")
        .is_some_and(|value| value == "1");
    let enabled = std::env::var_os("THRUPUT_HEAP_PROFILING")
        .is_some_and(|value| value == "1");
    #[cfg(not(all(target_os = "linux", feature = "heap-profiling")))]
    if enabled {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "THRUPUT_HEAP_PROFILING=1 needs a Linux scpd built with the heap-profiling feature",
        ));
    }
    // Leave allocator profiling controls untouched when sampling is disabled.
    #[cfg(all(target_os = "linux", feature = "heap-profiling"))]
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
    let Some(listener) = bind_listener(addr.as_deref(), allow_remote).await?
    else {
        return Ok(());
    };
    tracing::info!(addr = %listener.local_addr()?, "heap profile listener enabled");
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(async move {
                        let _ = serve_connection(
                            stream,
                            enabled,
                            allow_remote,
                            dump_heap,
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

#[cfg(all(target_os = "linux", feature = "heap-profiling"))]
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
        let profile = crate::heap_profile::jemalloc_to_pprof(
            &std::fs::read_to_string(&cleanup.0)?,
            &std::fs::read_to_string("/proc/self/maps")?,
            &mut LinuxSymbolizer::default(),
        );
        // Drop parsed debuginfo so it neither stays resident nor appears in
        // later profiles.
        backtrace::clear_symbol_cache();
        profile
    })
    .await
    .map_err(io::Error::other)?
}

#[cfg(all(target_os = "linux", feature = "heap-profiling"))]
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
#[cfg(all(target_os = "linux", feature = "heap-profiling"))]
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
#[cfg(all(target_os = "linux", feature = "heap-profiling"))]
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

#[cfg(all(target_os = "linux", feature = "heap-profiling"))]
#[derive(Default)]
struct LinuxSymbolizer {
    build_ids: std::collections::BTreeMap<String, Option<String>>,
}

#[cfg(all(target_os = "linux", feature = "heap-profiling"))]
impl crate::heap_profile::Symbolize for LinuxSymbolizer {
    fn frames(&mut self, pc: u64) -> Vec<crate::heap_profile::Frame> {
        let mut frames = Vec::new();
        backtrace::resolve(pc as *mut std::ffi::c_void, |symbol| {
            if let Some(name) = symbol.name() {
                frames.push(crate::heap_profile::Frame {
                    name: format!("{name:#}"),
                    system_name: String::from_utf8_lossy(name.as_bytes())
                        .into_owned(),
                    file: symbol
                        .filename()
                        .map(|path| path.display().to_string()),
                    line: symbol.lineno(),
                });
            }
        });
        frames
    }

    fn build_id(&mut self, path: &str) -> Option<String> {
        self.build_ids
            .entry(path.to_owned())
            .or_insert_with(|| {
                use object::Object;
                use std::fmt::Write;
                // Read only the headers and note, not the whole file.
                let file = std::fs::File::open(path).ok()?;
                if !file.metadata().ok()?.is_file() {
                    return None;
                }
                let data = object::ReadCache::new(file);
                let elf = object::read::elf::ElfFile64::<
                    object::Endianness,
                    _,
                >::parse(&data)
                .ok()?;
                let bytes = elf.build_id().ok()??;
                let mut id = String::new();
                for byte in bytes {
                    write!(&mut id, "{byte:02x}").ok()?;
                }
                Some(id)
            })
            .clone()
    }
}

#[cfg(not(all(target_os = "linux", feature = "heap-profiling")))]
async fn dump_heap(
    _permit: tokio::sync::OwnedSemaphorePermit,
) -> io::Result<Vec<u8>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "heap profiling unavailable",
    ))
}

fn check_listen_addrs(
    value: &str,
    addrs: &[SocketAddr],
    allow_remote: bool,
) -> io::Result<bool> {
    if addrs.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("SCPD_PPROF_LISTEN={value} resolves to no addresses"),
        ));
    }
    let remote = addrs
        .iter()
        .find(|addr| !addr.ip().to_canonical().is_loopback());
    if let Some(addr) = remote
        && !allow_remote
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "SCPD_PPROF_LISTEN={value} resolves to non-loopback address {}; the heap profile listener is unauthenticated. Set SCPD_PPROF_ALLOW_REMOTE=1 to allow it.",
                addr.ip()
            ),
        ));
    }
    Ok(remote.is_some())
}

async fn bind_listener(
    addr: Option<&str>,
    allow_remote: bool,
) -> io::Result<Option<TcpListener>> {
    let Some(addr) = addr else { return Ok(None) };
    let addrs: Vec<_> = tokio::net::lookup_host(addr).await?.collect();
    if check_listen_addrs(addr, &addrs, allow_remote)? {
        for addr in &addrs {
            if !addr.ip().to_canonical().is_loopback() {
                tracing::warn!(%addr, "heap profile listener is unauthenticated and reachable remotely");
            }
        }
    }
    TcpListener::bind(addrs.as_slice()).await.map(Some)
}

async fn serve_connection<F, Fut>(
    mut stream: TcpStream,
    enabled: bool,
    allow_remote: bool,
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
    // Decode only the header block; body bytes may follow in the same read.
    let headers = request
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .and_then(|end| std::str::from_utf8(&request[..end]).ok());
    let mut lines = headers.unwrap_or_default().split("\r\n");
    let parts: Vec<_> = lines
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .collect();
    let valid = valid && headers.is_some() && parts.len() == 3;
    let method = parts.first().copied().unwrap_or_default();
    let route = parts.get(1) == Some(&"/debug/pprof/heap")
        && matches!(parts.get(2), Some(&"HTTP/1.0" | &"HTTP/1.1"));
    let mut hosts = Vec::new();
    let mut origins = Vec::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            let value = value.trim_matches([' ', '\t']);
            if name.eq_ignore_ascii_case("Host") {
                hosts.push(value);
            } else if name.eq_ignore_ascii_case("Origin") {
                origins.push(value);
            }
        }
    }
    // Reject DNS names so browser DNS rebinding cannot expose heap profiles.
    let allowed = hosts.len() == 1
        && allowed_host(hosts[0], allow_remote)
        && (origins.is_empty()
            || (origins.len() == 1
                && origins[0].split_once("://").is_some_and(
                    |(scheme, rest)| {
                        !scheme.is_empty()
                            && rest
                                .split('/')
                                .next()
                                .unwrap_or_default()
                                .eq_ignore_ascii_case(hosts[0])
                    },
                )));
    let (status, body) = if !valid {
        ("400 Bad Request", Vec::new())
    } else if !enabled || !route {
        ("404 Not Found", Vec::new())
    } else if !matches!(method, "GET" | "HEAD") {
        ("405 Method Not Allowed", Vec::new())
    } else if !allowed {
        ("403 Forbidden", Vec::new())
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
    let allow = if status == "405 Method Not Allowed" {
        "Allow: GET, HEAD\r\n"
    } else {
        ""
    };
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n{allow}\r\n",
        body.len()
    );
    timeout(Duration::from_secs(10), async {
        stream.write_all(header.as_bytes()).await?;
        if method != "HEAD" {
            stream.write_all(&body).await?;
        }
        stream.shutdown().await
    })
    .await
    .map_err(|_| {
        io::Error::new(io::ErrorKind::TimedOut, "response write timeout")
    })?
}

fn allowed_host(value: &str, allow_remote: bool) -> bool {
    let host = if let Some(rest) = value.strip_prefix('[') {
        let Some((host, suffix)) = rest.split_once(']') else {
            return false;
        };
        if !suffix.is_empty() && !suffix.starts_with(':') {
            return false;
        }
        // Brackets hold only IPv6 literals, never `localhost`.
        return host
            .parse::<std::net::Ipv6Addr>()
            .is_ok_and(|ip| allow_remote || ip.to_canonical().is_loopback());
    } else if value.parse::<IpAddr>().is_ok() {
        value
    } else {
        value.rsplit_once(':').map_or(value, |(host, _)| host)
    };
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|ip| allow_remote || ip.to_canonical().is_loopback())
}

#[cfg(test)]
mod tests {
    use super::*;
    static TEST_LOCK: tokio::sync::Mutex<()> =
        tokio::sync::Mutex::const_new(());

    #[tokio::test]
    async fn unset_listen_address_creates_no_listener() {
        assert!(bind_listener(None, false).await.unwrap().is_none());
    }

    #[test]
    fn listen_addresses_require_loopback_or_opt_in() {
        for ip in ["127.0.0.1", "127.1.2.3", "::1", "::ffff:127.0.0.1"] {
            let addr = SocketAddr::new(ip.parse().unwrap(), 0);
            assert!(!check_listen_addrs(ip, &[addr], false).unwrap());
        }
        for ip in ["0.0.0.0", "::", "10.0.0.1", "192.168.1.1", "2001:db8::1"] {
            let addr = SocketAddr::new(ip.parse().unwrap(), 0);
            assert_eq!(
                check_listen_addrs(ip, &[addr], false).unwrap_err().kind(),
                io::ErrorKind::PermissionDenied
            );
            assert!(check_listen_addrs(ip, &[addr], true).unwrap());
        }
        let mixed = [
            "127.0.0.1:0".parse().unwrap(),
            "10.0.0.1:0".parse().unwrap(),
        ];
        assert_eq!(
            check_listen_addrs("mixed", &mixed, false)
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        assert!(check_listen_addrs("mixed", &mixed, true).unwrap());
        assert!(check_listen_addrs("empty", &[], true).is_err());
    }

    #[tokio::test]
    async fn listener_requires_remote_opt_in() {
        assert!(bind_listener(Some("localhost:0"), false).await.is_ok());
        let err = bind_listener(Some("0.0.0.0:0"), false).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        assert!(err.to_string().contains("SCPD_PPROF_ALLOW_REMOTE"));
        assert!(bind_listener(Some("0.0.0.0:0"), true).await.is_ok());
    }

    #[tokio::test]
    async fn startup_requires_remote_opt_in() {
        let _guard = TEST_LOCK.lock().await;
        // SAFETY: TEST_LOCK serializes tests that access these variables.
        unsafe {
            std::env::remove_var("THRUPUT_HEAP_PROFILING");
            std::env::remove_var("SCPD_PPROF_ALLOW_REMOTE");
            std::env::set_var("SCPD_PPROF_LISTEN", "0.0.0.0:0");
        }
        let refused = start_from_env().await;
        // SAFETY: the test still owns TEST_LOCK.
        unsafe { std::env::set_var("SCPD_PPROF_ALLOW_REMOTE", "1") };
        let allowed = start_from_env().await;
        // SAFETY: the test still owns TEST_LOCK. Clean up before assertions.
        unsafe {
            std::env::remove_var("SCPD_PPROF_LISTEN");
            std::env::remove_var("SCPD_PPROF_ALLOW_REMOTE");
            std::env::remove_var("THRUPUT_HEAP_PROFILING");
        }
        assert_eq!(
            refused.unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        allowed.unwrap();
    }

    async fn request(
        enabled: bool,
        line: &str,
        headers: &str,
        allow_remote: bool,
        dump_expected: bool,
        fail: bool,
    ) -> Vec<u8> {
        let _guard = TEST_LOCK.lock().await;
        let listener = bind_listener(Some("127.0.0.1:0"), false)
            .await
            .unwrap()
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            serve_connection(
                stream,
                enabled,
                allow_remote,
                |permit| async move {
                    let _permit = permit;
                    assert!(dump_expected, "rejected request must never dump");
                    if fail {
                        Err(io::Error::other("test dump failure"))
                    } else {
                        Ok(b"profile protobuf".to_vec())
                    }
                },
            )
            .await
            .unwrap();
        });
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(
                format!(
                    "{line}\r\n{}\r\n",
                    headers.replace("<port>", &addr.port().to_string())
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        task.await.unwrap();
        response
    }

    #[tokio::test]
    async fn disabled_heap_endpoint_is_404() {
        let response = request(
            false,
            "GET /debug/pprof/heap HTTP/1.1",
            "Host: 127.0.0.1:<port>\r\n",
            false,
            false,
            false,
        )
        .await;
        assert!(response.starts_with(b"HTTP/1.1 404 Not Found\r\n"));
    }

    #[tokio::test]
    async fn enabled_heap_endpoint_serves_profile() {
        let response = request(
            true,
            "GET /debug/pprof/heap HTTP/1.1",
            "Host: 127.0.0.1:<port>\r\n",
            false,
            true,
            false,
        )
        .await;
        assert!(response.starts_with(b"HTTP/1.1 200 OK\r\n"));
        assert!(response.ends_with(b"profile protobuf"));
        assert!(
            String::from_utf8_lossy(&response)
                .contains("Content-Length: 16\r\n")
        );
    }

    #[tokio::test]
    async fn other_routes_and_versions_are_404() {
        for line in [
            "GET /debug/pprof/heap/ HTTP/1.1",
            "GET /debug/pprof/profile HTTP/1.1",
            "GET /debug/pprof/heap HTTP/2.0",
        ] {
            // A failing dump would yield 500 if the route incorrectly matched.
            assert!(
                request(true, line, "", false, false, true)
                    .await
                    .starts_with(b"HTTP/1.1 404 Not Found\r\n")
            );
        }
    }

    #[tokio::test]
    async fn host_and_origin_guard() {
        for (headers, allow_remote, allowed) in [
            ("Host: evil.example:<port>\r\n", false, false),
            ("Host: 127.0.0.1:<port>\r\n", false, true),
            ("Host: localhost:<port>\r\n", false, true),
            ("Host: [::1]:<port>\r\n", false, true),
            ("Host: LOCALHOST\r\n", false, true),
            ("hOsT:\t127.0.0.1:<port> \t\r\n", false, true),
            ("Host: ::ffff:127.0.0.1\r\n", false, true),
            ("Host: [::1]invalid\r\n", false, false),
            ("Host: [localhost]:<port>\r\n", false, false),
            ("Host: [127.0.0.1]\r\n", false, false),
            ("Host: localhost.\r\n", false, false),
            ("Host: foo.localhost\r\n", false, false),
            ("Host: [::1%lo0]\r\n", false, false),
            ("Host: [2001:db8::1]:9648\r\n", true, true),
            (
                "Host: [::1]:<port>\r\nOrigin: http://[::1]:<port>\r\n",
                false,
                true,
            ),
            ("Host: localhost\r\nOrigin: ://localhost\r\n", false, false),
            ("Host: \r\n", true, false),
            ("Host: 10.0.0.5:9648\r\n", true, true),
            ("Host: evil.example\r\n", true, false),
            ("Host: 10.0.0.5:9648\r\n", false, false),
            (
                "Host: 127.0.0.1:<port>\r\nOrigin: http://evil.example\r\n",
                false,
                false,
            ),
            (
                "Host: 127.0.0.1:<port>\r\nOrigin: http://127.0.0.1:<port>\r\n",
                false,
                true,
            ),
            (
                "Host: LOCALHOST\r\nOrigin: http://localhost/path\r\n",
                false,
                true,
            ),
            ("Host: 127.0.0.1:<port>\r\nOrigin: null\r\n", false, false),
            (
                "Host: localhost\r\nOrigin: http://localhost\r\nOrigin: http://localhost\r\n",
                false,
                false,
            ),
            ("", false, false),
            ("Host: localhost\r\nHost: localhost\r\n", false, false),
        ] {
            let response = request(
                true,
                "GET /debug/pprof/heap HTTP/1.1",
                headers,
                allow_remote,
                allowed,
                false,
            )
            .await;
            let status = if allowed { "200 OK" } else { "403 Forbidden" };
            assert!(
                response
                    .starts_with(format!("HTTP/1.1 {status}\r\n").as_bytes()),
                "{headers}"
            );
        }
        assert!(
            request(
                true,
                "GET /debug/pprof/heap HTTP/1.0",
                "",
                false,
                false,
                false
            )
            .await
            .starts_with(b"HTTP/1.1 403 Forbidden\r\n")
        );
    }

    #[tokio::test]
    async fn methods_and_invalid_requests() {
        let response = request(
            true,
            "POST /debug/pprof/heap HTTP/1.1",
            "",
            false,
            false,
            false,
        )
        .await;
        assert!(response.starts_with(b"HTTP/1.1 405 Method Not Allowed\r\n"));
        assert!(
            String::from_utf8_lossy(&response)
                .contains("\r\nAllow: GET, HEAD\r\n")
        );
        let response = request(
            true,
            "HEAD /debug/pprof/heap HTTP/1.1",
            "Host: 127.0.0.1:<port>\r\n",
            false,
            true,
            false,
        )
        .await;
        assert!(response.starts_with(b"HTTP/1.1 200 OK\r\n"));
        assert!(
            String::from_utf8_lossy(&response)
                .contains("Content-Length: 16\r\n")
        );
        assert!(response.ends_with(b"\r\n\r\n"));
        for line in ["GET", "GET /debug/pprof/heap HTTP/1.1 extra"] {
            assert!(
                request(true, line, "", false, false, false)
                    .await
                    .starts_with(b"HTTP/1.1 400 Bad Request\r\n")
            );
        }
    }

    #[tokio::test]
    async fn oversized_and_unterminated_requests_are_400() {
        let _guard = TEST_LOCK.lock().await;
        for request in [
            b"GET /debug/pprof/heap HTTP/1.1\r\nHost: localhost\r\n".to_vec(),
            vec![b'x'; 8193],
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let task = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                serve_connection(stream, true, false, |_| async {
                    panic!("invalid request must never dump")
                })
                .await
                .unwrap();
            });
            let mut stream = TcpStream::connect(addr).await.unwrap();
            stream.write_all(&request).await.unwrap();
            stream.shutdown().await.unwrap();
            let mut response = Vec::new();
            stream.read_to_end(&mut response).await.unwrap();
            assert!(response.starts_with(b"HTTP/1.1 400 Bad Request\r\n"));
            task.await.unwrap();
        }
    }

    #[tokio::test]
    async fn dump_failure_is_500() {
        assert!(
            request(
                true,
                "GET /debug/pprof/heap HTTP/1.1",
                "Host: 127.0.0.1:<port>\r\n",
                false,
                true,
                true
            )
            .await
            .starts_with(b"HTTP/1.1 500 Internal Server Error\r\n")
        );
    }

    #[tokio::test]
    async fn concurrent_request_is_429_and_long_dump_survives() {
        let _guard = TEST_LOCK.lock().await;
        let listener = bind_listener(Some("127.0.0.1:0"), false)
            .await
            .unwrap()
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let (started, ready) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let first = tokio::spawn(serve_connection(
                stream,
                true,
                false,
                |permit| async move {
                    let _permit = permit;
                    started.send(()).unwrap();
                    tokio::time::sleep(Duration::from_secs(11)).await;
                    Ok(b"long profile".to_vec())
                },
            ));
            let (stream, _) = listener.accept().await.unwrap();
            serve_connection(stream, true, false, |_| async {
                panic!("busy request must not dump")
            })
            .await
            .unwrap();
            first.await.unwrap().unwrap();
        });
        let mut first = TcpStream::connect(addr).await.unwrap();
        let request = format!(
            "GET /debug/pprof/heap HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
            addr.port()
        );
        first.write_all(request.as_bytes()).await.unwrap();
        ready.await.unwrap();
        let mut second = TcpStream::connect(addr).await.unwrap();
        second.write_all(request.as_bytes()).await.unwrap();
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

    #[cfg(not(all(target_os = "linux", feature = "heap-profiling")))]
    #[tokio::test]
    async fn activation_requires_linux_feature_build() {
        let _guard = TEST_LOCK.lock().await;
        // SAFETY: TEST_LOCK serializes tests that access this variable.
        unsafe { std::env::set_var("THRUPUT_HEAP_PROFILING", "1") };
        let result = start_from_env().await;
        // SAFETY: the test still owns TEST_LOCK.
        unsafe { std::env::remove_var("THRUPUT_HEAP_PROFILING") };
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Unsupported);
    }

    #[cfg(all(target_os = "linux", feature = "heap-profiling"))]
    #[inline(never)]
    fn retain_heap_profile_marker() -> Box<[u8]> {
        std::hint::black_box(vec![42; 8 * 1024 * 1024].into_boxed_slice())
    }

    #[cfg(all(target_os = "linux", feature = "heap-profiling"))]
    #[tokio::test]
    async fn jemalloc_starts_inactive_and_can_dump_pprof() {
        let _guard = TEST_LOCK.lock().await;
        assert!(!read_bool(c"prof.active").unwrap());
        set_active(true).unwrap();
        let allocation = retain_heap_profile_marker();
        std::hint::black_box(&allocation);
        let profile =
            dump_heap(DUMP_SLOT.clone().acquire_owned().await.unwrap()).await;
        // Restore shared allocator state even if the conversion failed.
        set_active(false).unwrap();
        let profile = profile.unwrap();
        std::hint::black_box(&allocation);
        use crate::heap_profile::wire::decode;
        let fields = decode(&profile);
        let strings: Vec<_> =
            fields.iter().filter(|f| f.0 == 6).map(|f| f.2).collect();
        assert!(fields.iter().filter(|f| f.0 == 5).any(|f| {
            decode(f.2).iter().any(|f| {
                f.0 == 2
                    && String::from_utf8_lossy(
                        strings[usize::try_from(f.1).unwrap()],
                    )
                    .contains("retain_heap_profile_marker")
            })
        }));
        let executable = std::env::current_exe().unwrap();
        let path = executable.to_str().unwrap().as_bytes();
        assert!(fields.iter().filter(|f| f.0 == 3).any(|f| {
            let mapping = decode(f.2);
            mapping.iter().any(|f| {
                f.0 == 5 && strings[usize::try_from(f.1).unwrap()] == path
            }) && mapping.iter().any(|f| {
                f.0 == 6 && !strings[usize::try_from(f.1).unwrap()].is_empty()
            })
        }));
    }
}
