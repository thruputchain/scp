use async_trait::async_trait;
use scp_kernel::*;
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;
use tokio::sync::RwLockReadGuard as TokioRwLockReadGuard;
use tokio::sync::RwLockWriteGuard as TokioRwLockWriteGuard;
use tokio::sync::mpsc::{Receiver, Sender};
use xdr_common::*;

// rpc --async--> controller service --async----sync--> kernel service

struct ControllerService(Arc<TokioRwLock<ControllerBridge>>);

struct ControllerBridge {
    pub tx_request: Sender<ControlRequest>,
    pub rx_reply: Receiver<ControlReply>,
}

impl ControllerService {
    fn new(
        tx_request: Sender<ControlRequest>,
        rx_reply: Receiver<ControlReply>,
    ) -> ControllerService {
        ControllerService(Arc::new(TokioRwLock::new(ControllerBridge {
            tx_request,
            rx_reply,
        })))
    }

    async fn lock(&self) -> TokioRwLockReadGuard<'_, ControllerBridge> {
        self.0.read().await
    }

    async fn lock_mut(&self) -> TokioRwLockWriteGuard<'_, ControllerBridge> {
        self.0.write().await
    }
}

enum ControlRequest {
    Nominate(SCPSlotIndex, SCPValue, SCPValue),
    StopNomination(SCPSlotIndex),
    UpdateLocalQuorumSet(SCPQuorumSet),
    ReceivedEnvelope(SCPEnvelope),
}

enum ControlReply {
    Nominate(bool),
    StopNomination(),
    UpdateLocalQuorumSet(),
    ReceivedEnvelope(),
}

#[async_trait]
impl proto::SCP_Control_SCP_Control1 for ControllerService {
    async fn scp_control_null(&self) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_control_nominate(
        &self,
        slot_index: SCPSlotIndex,
        value: SCPValue,
        prev_value: SCPValue,
    ) -> Result<bool, ::rpc_driver::ProgramError> {
        let mut x = self.lock_mut().await;
        x.tx_request
            .send(ControlRequest::Nominate(slot_index, value, prev_value))
            .await
            .unwrap();
        if let ControlReply::Nominate(r) = x.rx_reply.recv().await.unwrap() {
            Ok(r)
        } else {
            panic!("bug")
        }
    }

    async fn scp_control_stop_nomination(
        &self,
        slot_index: SCPSlotIndex,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        let mut x = self.lock_mut().await;
        x.tx_request
            .send(ControlRequest::StopNomination(slot_index))
            .await
            .unwrap();
        if let ControlReply::StopNomination() = x.rx_reply.recv().await.unwrap() {
            Ok(())
        } else {
            panic!("bug")
        }
    }

    async fn scp_control_update_local_quorum_set(
        &self,
        qset: SCPQuorumSet,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        let mut x = self.lock_mut().await;
        x.tx_request
            .send(ControlRequest::UpdateLocalQuorumSet(qset))
            .await
            .unwrap();
        if let ControlReply::UpdateLocalQuorumSet() = x.rx_reply.recv().await.unwrap() {
            Ok(())
        } else {
            panic!("bug")
        }
    }

    async fn scp_control_received_envelope(
        &self,
        env: SCPEnvelope,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        let mut x = self.lock_mut().await;
        x.tx_request
            .send(ControlRequest::ReceivedEnvelope(env))
            .await
            .unwrap();
        if let ControlReply::ReceivedEnvelope() = x.rx_reply.recv().await.unwrap() {
            Ok(())
        } else {
            panic!("bug")
        }
    }
}
