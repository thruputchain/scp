//  rpc --> [ async ControlService ] ==>
//      [ sync ControlKernel --> sync kernel  --> HandleKernel ] ==>
//          [ async HandleClient ] --> rpc

use crate::controller::*;
use crate::handler::*;
use core::net::SocketAddr;
use rpc_driver::*;
use scp_kernel::*;
use tokio;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

pub async fn start_server(
    handler_sockaddr: SocketAddr,
    bind_controller_sockaddr: SocketAddr,
    cfg: proto::SCPConfig,
    secret_key: &proto::SecretKey,
    is_validator: bool,
) -> SocketAddr {
    let handle_client =
        proto::SCP_Handle_SCP_Handle1_Client::new(connect_client(&handler_sockaddr).await.unwrap());
    let control_service = start_service(handle_client, cfg, secret_key, is_validator).await;
    start_control_server(&bind_controller_sockaddr, control_service).await
}

async fn start_service(
    handle_rpc_client: SCP_Handle_SCP_Handle1_Client,
    cfg: proto::SCPConfig,
    secret_key: &proto::SecretKey,
    is_validator: bool,
) -> ControlService {
    // construct

    let (tx_switch_timer, rx_switch_timer) = mpsc::unbounded_channel::<SwitchTimer>();

    let (tx_handle_request, rx_handle_request) = mpsc::channel::<HandleRequest>(1);
    let (tx_handle_reply, rx_handle_reply) = mpsc::channel::<HandleReply>(1);

    let handle_kernel = HandleKernel::new(
        tx_handle_request,
        rx_handle_reply,
        tx_switch_timer,
        &cfg,
        secret_key,
    );
    let handle_client = HandleClient::new(rx_handle_request, tx_handle_reply, handle_rpc_client);

    let kernel = StellarKernel::<HandleKernel, HandleKernel>::new(
        derive_public_key(secret_key),
        is_validator,
        cfg.quorum_set,
        handle_kernel.clone(),
        handle_kernel.clone(),
    );

    let (tx_control_request, rx_control_request) = mpsc::channel::<ControlRequest>(1);
    let (tx_control_reply, rx_control_reply) = mpsc::channel::<ControlReply>(1);

    let control_kernel = ControlKernel::new(
        rx_control_request,
        tx_control_reply,
        rx_switch_timer,
        kernel,
        handle_kernel,
    )
    .await;

    let control_service = ControlService::new(tx_control_request, rx_control_reply);

    // start

    handle_client.start().await;
    control_kernel.start();

    control_service
}

async fn start_control_server<S: SCP_Control_SCP_Control1 + 'static>(
    addr: &SocketAddr,
    svc: S,
) -> SocketAddr {
    let listener = TcpListener::bind(addr).await.unwrap();
    let sockaddr = listener.local_addr().unwrap();

    let mut service_map = ServiceMap::new();
    service_map.add_service(Box::new(
        groundhog_scp::SCP_Control_SCP_Control1_Dispatcher::new(Box::new(svc)),
    ));

    RpcServer::new(service_map, listener).start().await;

    sockaddr
}
