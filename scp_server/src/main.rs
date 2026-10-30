use clap::Parser;
use core::net::SocketAddr;
use scp_kernel as _;
use thruput_rpc_driver::{
    ConnectionConfig, RpcServer, ServiceMap, connect_client,
};
use tokio::net::{TcpListener, TcpSocket};
use tracing::info;
use xdr_common::*;

mod config_shim;
use config_shim::SCPConfigShim;

mod system;
use system::*;

mod controller;
mod handler;
#[cfg_attr(
    not(all(target_os = "linux", feature = "heap-profiling")),
    allow(dead_code)
)]
mod heap_profile;
mod heap_profiling;
mod telemetry;

#[cfg(all(target_os = "linux", feature = "heap-profiling"))]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

// SAFETY: jemalloc reads this NUL-terminated configuration at initialization.
#[cfg(all(target_os = "linux", feature = "heap-profiling"))]
#[unsafe(export_name = "malloc_conf")]
pub static MALLOC_CONF: &[u8] =
    b"prof:true,prof_active:false,lg_prof_sample:19\0";

#[derive(Parser, Debug)]
#[command(
    version,
    about = "SCP Server - Stellar Consensus Protocol stand-alone server",
    long_about = "Use `RUST_LOG=warn|info|debug|trace` to set logging level. Set OTEL_EXPORTER_OTLP_ENDPOINT to enable tracing and OTEL_SPAN_FILTER to filter spans."
)]
struct Args {
    #[arg(
        long,
        default_value = "127.0.0.1:11055",
        help = "Bind address for the SCP control API"
    )]
    control_addr: String,

    #[arg(
        long,
        default_value = "127.0.0.1:22022",
        help = "Connect address for the SCP handler API"
    )]
    handle_addr: String,

    #[arg(
        long,
        default_value_t = true,
        action = clap::ArgAction::Set,
        help = "Enable SO_REUSEADDR and SO_REUSEPORT when binding the SCP control API"
    )]
    reuse: bool,

    #[arg(
        long,
        default_value = "false",
        value_parser = clap::value_parser!(bool),
        help = "Is this SCP node a validator"
    )]
    is_validator: Option<bool>,

    #[arg(
        long,
        default_value = "config.json",
        help = "Path to JSON config file with schema according to scp_config.x"
    )]
    config_path: String,

    #[arg(
        long,
        default_value = "",
        help = "Base64-std encoded secret key for this SCP node. Prefer --secret-key-file: command-line arguments are readable by every process on the host"
    )]
    secret_key: String,

    #[arg(
        long,
        conflicts_with_all = ["secret_key", "secret_key_fd"],
        help = "Path to a file holding the base64-std encoded secret key for this SCP node, surrounding whitespace ignored"
    )]
    secret_key_file: Option<String>,

    #[arg(
        long,
        conflicts_with_all = ["secret_key", "secret_key_file"],
        help = "Inherited file descriptor to read the base64-std encoded secret key from, up to EOF; for launchers, which can pass a pipe that nothing else can read"
    )]
    secret_key_fd: Option<i32>,
}

/// The secret key as base64 text, from --secret-key-file or --secret-key-fd
/// when given, else from --secret-key (which may be empty).
fn secret_key_text(args: &Args) -> String {
    use std::io::Read;
    let text = if let Some(path) = &args.secret_key_file {
        std::fs::read_to_string(path).unwrap_or_else(|e| {
            panic!("failed to read secret key file {path}: {e}")
        })
    } else if let Some(fd) = args.secret_key_fd {
        assert!(fd >= 0, "invalid secret key file descriptor {fd}");
        // SAFETY: the launcher handed us this descriptor to read; taking
        // ownership here closes it once the key is read.
        let mut file = unsafe {
            <std::fs::File as std::os::fd::FromRawFd>::from_raw_fd(fd)
        };
        let mut text = String::new();
        file.read_to_string(&mut text).unwrap_or_else(|e| {
            panic!("failed to read secret key from fd {fd}: {e}")
        });
        text
    } else {
        args.secret_key.clone()
    };
    text.trim().to_string()
}

#[tokio::main]
async fn main() {
    use opentelemetry::trace::TracerProvider;
    use tracing_subscriber::prelude::*;

    let args = Args::parse();
    let secret_key_text = secret_key_text(&args);
    let instance_id = scp_util::parse_secret_key_from_base64(&secret_key_text)
        .ok()
        .and_then(|key| match scp_util::derive_node_id(&key) {
            thruput_xdr::types::PublicKey::Ed25519 { ed25519 } => Some(
                ed25519
                    .into_inner()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect(),
            ),
            _ => None,
        });
    let (provider, mut warnings) = telemetry::provider(instance_id);
    let (span_filter, warning) = telemetry::span_filter();
    warnings.extend(warning);
    let otel_layer = provider.as_ref().map(|provider| {
        tracing_opentelemetry::layer()
            .with_tracer(provider.tracer("scpd"))
            .with_filter(span_filter)
    });
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_span_events(
            tracing_subscriber::fmt::format::FmtSpan::NEW
                | tracing_subscriber::fmt::format::FmtSpan::CLOSE,
        )
        .event_format(tracing_subscriber::fmt::format())
        // .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        .with_ansi(false);
    tracing_subscriber::registry()
        .with(
            fmt_layer
                .with_filter(tracing_subscriber::EnvFilter::from_default_env()),
        )
        .with(otel_layer)
        .init();
    // Error level so a misconfiguration shows under the default filter.
    for error in warnings {
        tracing::error!(%error, "OpenTelemetry tracing configuration problem");
    }

    if let Err(err) = heap_profiling::start_from_env().await {
        eprintln!("scpd: failed to initialize heap profiling: {err}");
        std::process::exit(1);
    }

    let termination = telemetry::termination_signal();
    telemetry::run_until_termination(run(args), termination).await;
    if let Some(provider) = provider {
        let termination = telemetry::termination_signal();
        // Shutdown drains the batch processor; keep its blocking wait off Tokio.
        let flush = async move {
            let _ = tokio::task::spawn_blocking(move || {
                provider
                    .shutdown_with_timeout(std::time::Duration::from_secs(5))
            })
            .await;
        };
        if !telemetry::flush_unless_interrupted(flush, termination).await {
            std::process::exit(1);
        }
    }
}

async fn run(args: Args) {
    info!(
        "control_addr={} handle_addr={} reuse={}",
        args.control_addr, args.handle_addr, args.reuse
    );

    // TODO: remove the shim when XDR-JSON is available
    // parse the file args.config_path from JSON as an SCPConfig_shim value;
    // convert that value into SCPConfig and assign it to a variable called cfg.
    let config_json = tokio::fs::read_to_string(&args.config_path)
        .await
        .unwrap_or_else(|e| {
            panic!("Failed to read config file '{}': {}", args.config_path, e)
        });

    let config_shim: SCPConfigShim = serde_json::from_str(&config_json)
        .unwrap_or_else(|e| panic!("Failed to parse config JSON: {}", e));

    let cfg = config_shim.into_scp_config();

    // handle client

    let handle_socket_addr: SocketAddr = args.handle_addr.parse().unwrap();
    let handle_client = thruput_scp::SCP_Handle_SCP_Handle1_Client::new(
        connect_client(&handle_socket_addr)
            .await
            .unwrap()
            .into_handle(),
    );

    // create server

    let secret_key_text = secret_key_text(&args);
    assert!(!secret_key_text.is_empty(), "no secret key provided");
    let secret_key = scp_util::parse_secret_key_from_base64(&secret_key_text)
        .unwrap_or_else(|e| panic!("failed to parse secret key: {}", e));

    let service = start_service(
        handle_client,
        cfg,
        &secret_key,
        args.is_validator.unwrap(),
    )
    .await;

    // control server

    let control_socket_addr: SocketAddr = args.control_addr.parse().unwrap();
    let control_listener =
        bind_control_listener(control_socket_addr, args.reuse)
            .await
            .unwrap();

    let mut control_service_map = ServiceMap::new();
    control_service_map.add_service(Box::new(
        thruput_scp::SCP_Control_SCP_Control1_Dispatcher::new(Box::new(
            service,
        )),
    ));

    // Cap incoming control RPC messages at 10 MiB.
    const CONTROL_MAX_MESSAGE_SIZE: u32 = 10 * 1024 * 1024;
    let control_config = ConnectionConfig::new(
        Some(CONTROL_MAX_MESSAGE_SIZE),
        control_service_map,
    );
    let control_server =
        RpcServer::new_with_config(control_listener, control_config);
    control_server.start().await;
}

async fn bind_control_listener(
    addr: SocketAddr,
    reuse: bool,
) -> std::io::Result<TcpListener> {
    if !reuse {
        return TcpListener::bind(addr).await;
    }

    let socket = match addr {
        SocketAddr::V4(_) => TcpSocket::new_v4()?,
        SocketAddr::V6(_) => TcpSocket::new_v6()?,
    };

    socket.set_reuseaddr(true)?;
    #[cfg(unix)]
    socket.set_reuseport(true)?;
    socket.bind(addr)?;
    socket.listen(1024)
}

#[cfg(test)]
mod system_test;
