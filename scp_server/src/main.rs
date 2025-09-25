use clap::Parser;
use core::net::SocketAddr;
use futures::future;
use rpc_driver::{RpcServer, connect_client, server_state::ServiceMap};
use tokio;
use tokio::net::TcpListener;
use tracing::info;
use xdr_common::*;

mod config_shim;
use config_shim::SCPConfigShim;

mod system;
use system::*;

mod controller;
mod handler;

#[derive(Parser, Debug)]
#[command(version, about = "SCP Server - Stellar Consensus Protocol stand-alone server", long_about = None)]
struct Args {
    #[arg(
        long,
        default_value = ":11055",
        help = "Bind address for the SCP control API"
    )]
    control_addr: String,

    #[arg(
        long,
        default_value = "localhost:22022",
        help = "Connect address for the SCP handler API"
    )]
    handle_addr: String,

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
        help = "Base64-std encoded secret key for this SCP node"
    )]
    secret_key: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    info!(
        "control_addr={} handle_addr={}",
        args.control_addr, args.handle_addr
    );

    // TODO: remove the shim when XDR-JSON is available
    // parse the file args.config_path from JSON as an SCPConfig_shim value;
    // convert that value into SCPConfig and assign it to a variable called cfg.
    let config_json = tokio::fs::read_to_string(&args.config_path)
        .await
        .unwrap_or_else(|e| panic!("Failed to read config file '{}': {}", args.config_path, e));

    let config_shim: SCPConfigShim = serde_json::from_str(&config_json)
        .unwrap_or_else(|e| panic!("Failed to parse config JSON: {}", e));

    let cfg = config_shim.to_scp_config();

    // handle client

    let handle_socket_addr: SocketAddr = args.handle_addr.parse().unwrap();
    let handle_client = proto::SCP_Handle_SCP_Handle1_Client::new(
        connect_client(&handle_socket_addr).await.unwrap(),
    );

    // create server

    assert!(!args.secret_key.is_empty(), "no secret key provided");
    let secret_key = proto::parse_secret_key_from_base64(&args.secret_key)
        .unwrap_or_else(|e| panic!("failed to parse secret key: {}", e));

    let service = start_service(handle_client, cfg, &secret_key, args.is_validator.unwrap()).await;

    // control server

    let control_listener = TcpListener::bind(args.control_addr).await.unwrap();

    let mut control_service_map = ServiceMap::new();
    control_service_map.add_service(Box::new(
        groundhog_scp::SCP_Control_SCP_Control1_Dispatcher::new(Box::new(service)),
    ));

    let control_server = RpcServer::new(control_service_map, control_listener);
    control_server.start().await;

    future::pending::<()>().await;
}

#[cfg(test)]
mod system_test;
