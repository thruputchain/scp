//  rpc --a--> √ControlService --a--s--> ControlKernel --s-->
//    kernel --s-->
//      √HandleKernel --s--a--> √HandleClient --a--> rpc

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

#[derive(Debug, Clone)]
struct DriverService(Arc<StdRwLock<DriverBridge>>);

#[derive(Debug)]
struct DriverBridge {
    // controller
    pub rx_control_request: Receiver<ControlRequest>,
    pub tx_control_reply: Sender<ControlReply>,
    // handler
    pub tx_handle_request: Sender<HandleRequest>,
    pub rx_handle_reply: Receiver<HandleReply>,
}

impl DriverService {
    pub fn new(
        rx_control_request: Receiver<ControlRequest>,
        tx_control_reply: Sender<ControlReply>,
        tx_handle_request: Sender<HandleRequest>,
        rx_handle_reply: Receiver<HandleReply>,
    ) -> DriverService {
        DriverService(Arc::new(StdRwLock::new(DriverBridge {
            rx_control_request,
            tx_control_reply,
            tx_handle_request,
            rx_handle_reply,
        })))
    }
}
