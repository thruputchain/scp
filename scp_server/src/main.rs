use clap::Parser;
use core::net::SocketAddr;
use rpc_driver::{RpcServer, connect_client, server_state::ServiceMap, tcp_connection::accept};
use tokio;
use tokio::net::TcpListener;
use tracing::info;

mod server;
use server::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = ":30803")]
    peer_addr: String,

    #[arg(short, long, default_value = ":11055")]
    control_addr: String,

    #[arg(short, long, default_value = ":22022")]
    handle_addr: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    info!(
        "peer_addr={} control_addr={} handle_addr={}",
        args.peer_addr, args.control_addr, args.handle_addr
    );

    // control server

    let control_listener = TcpListener::bind(args.control_addr).await.unwrap();
    let control_addr = control_listener.local_addr().unwrap();

    let mut control_service_map = ServiceMap::new();
    // control_service_map.add_service(Box::new(TestProgram_TestVersion_Dispatcher {
    //     inner: Box::new(EchoCallbackSerivce {}),
    // }));

    // client
    let handle_socket_addr: SocketAddr = args.handle_addr.parse().unwrap();
    // let client =
    //     Scp_control_Scp_control1_Client::new(connect_client(&handle_socket_addr).await.unwrap());

    //XXX: start control server
    // let control_server = RpcServer::new(control_service_map, control_listener);
    // control_server.start().await;

    // peer server

    let peer_listener = TcpListener::bind(args.peer_addr).await.unwrap();
    let peer_addr = peer_listener.local_addr().unwrap();

    let mut peer_service_map = ServiceMap::new();
    // peer_service_map.add_service(Box::new(TestProgram_TestVersion_Dispatcher {
    //     inner: Box::new(EchoCallbackSerivce {}),
    // }));

    // server
    let peer_server = RpcServer::new(peer_service_map, peer_listener);
    peer_server.start().await;

    //
}
