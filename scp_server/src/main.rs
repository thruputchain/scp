use clap::{ArgAction, Parser};
use core::net::SocketAddr;
use futures::future;
use proto::*;
use rpc_driver::{RpcServer, connect_client, server_state::ServiceMap};
use tokio;
use tokio::net::TcpListener;
use tracing::info;
use xdr_common::*;

mod system;
use system::*;

mod controller;
mod handler;

#[derive(Parser, Debug)]
#[command(version, about = "SCP Server - Stellar Consensus Protocol stand-alone server", long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        default_value = ":11055",
        help = "Bind address for the SCP control API"
    )]
    control_addr: String,

    #[arg(
        short,
        long,
        default_value = "localhost:22022",
        help = "Connect address for the SCP handler API"
    )]
    handle_addr: String,

    #[arg(
        short,
        long,
        action = ArgAction::SetTrue,
        help = "Is this SCP node a validator"
    )]
    is_validator: bool,

    #[arg(
        short,
        long,
        default_value = "config.json",
        help = "Path to JSON config file with schema according to scp_config.x"
    )]
    config_path: String,

    #[arg(
        short,
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

    // XXX: parse config
    let qset = SCPQuorumSet {
        threshold: 0,
        validators: Default::default(),
        inner_sets: Default::default(),
    };
    let cfg = SCPConfig {
        max_timeout_ms: 1000,
        initial_ballot_timeout_ms: 750,
        increment_ballot_timeout_ms: 750,
        initial_nomination_timeout_ms: 750,
        increment_nomination_timeout_ms: 750,
        quorum_set: qset.clone(),
    };

    // handle client

    let handle_socket_addr: SocketAddr = args.handle_addr.parse().unwrap();
    let handle_client = proto::SCP_Handle_SCP_Handle1_Client::new(
        connect_client(&handle_socket_addr).await.unwrap(),
    );

    // create server

    assert!(!args.secret_key.is_empty(), "no secret key provided");
    let secret_key = proto::parse_secret_key_from_base64(&args.secret_key)
        .unwrap_or_else(|e| panic!("failed to parse secret key: {}", e));

    let service = start_service(handle_client, cfg, &secret_key, args.is_validator).await;

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
