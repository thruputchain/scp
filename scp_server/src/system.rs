//  rpc --> [ async ControlService ] ==>
//      [ sync ControlKernel --> sync kernel  --> HandleKernel ] ==>
//          [ async HandleClient ] --> rpc

use async_trait::async_trait;
use futures::future::join_all;
use scp_kernel::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tracing::info;

use std::sync::RwLock as StdRwLock;
use std::sync::RwLockReadGuard as StdRwLockReadGuard;
use std::sync::RwLockWriteGuard as StdRwLockWriteGuard;

use tokio::sync::RwLock as TokioRwLock;
use tokio::sync::RwLockReadGuard as TokioRwLockReadGuard;
use tokio::sync::RwLockWriteGuard as TokioRwLockWriteGuard;

use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::sleep;
use xdr_common::*;

use crate::controller::*;
use crate::handler::*;

pub async fn launch(
    handle: SCP_Handle_SCP_Handle1_Client,
    cfg: proto::SCPConfig,
    secret_key: &proto::SecretKey,
    is_validator: bool,
) {
    let (tx_handle_request, mut rx_handle_request) = mpsc::channel::<HandleRequest>(1);
    let (tx_handle_reply, mut rx_handle_reply) = mpsc::channel::<HandleReply>(1);
    let handle_kernel = HandleKernel::new(tx_handle_request, rx_handle_reply, &cfg, secret_key);
    let handle_client = HandleClient::new(rx_handle_request, tx_handle_reply, handle);

    let kernel = StellarKernel::<HandleKernel, HandleKernel>::new(
        derive_public_key(secret_key),
        is_validator,
        cfg.quorum_set,
        handle_kernel.clone(),
        handle_kernel.clone(),
    );

    XXX
}
