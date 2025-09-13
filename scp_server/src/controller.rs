use async_trait::async_trait;
use scp_kernel::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::sync::RwLockReadGuard as StdRwLockReadGuard;
use std::sync::RwLockWriteGuard as StdRwLockWriteGuard;
use std::thread;
use std::time::Duration;
use tokio::sync::RwLock as TokioRwLock;
use tokio::sync::RwLockReadGuard as TokioRwLockReadGuard;
use tokio::sync::RwLockWriteGuard as TokioRwLockWriteGuard;
use tokio::sync::mpsc::{Receiver, Sender, UnboundedReceiver, error::TryRecvError};
use tokio::sync::oneshot;
use tokio::time::sleep;
use xdr_common::*;

pub struct ControlService(Arc<TokioRwLock<ControlServiceState>>);

struct ControlServiceState {
    pub tx_request: Sender<ControlRequest>,
    pub rx_reply: Receiver<ControlReply>,
}

impl ControlService {
    pub fn new(
        tx_request: Sender<ControlRequest>,
        rx_reply: Receiver<ControlReply>,
    ) -> ControlService {
        ControlService(Arc::new(TokioRwLock::new(ControlServiceState {
            tx_request,
            rx_reply,
        })))
    }

    async fn lock(&self) -> TokioRwLockReadGuard<'_, ControlServiceState> {
        self.0.read().await
    }

    async fn lock_mut(&self) -> TokioRwLockWriteGuard<'_, ControlServiceState> {
        self.0.write().await
    }
}

pub enum ControlRequest {
    Nominate(SCPSlotIndex, SCPValue, SCPValue),
    StopNomination(SCPSlotIndex),
    UpdateLocalQuorumSet(SCPQuorumSet),
    ReceivedEnvelope(SCPEnvelope),
}

pub enum ControlReply {
    Nominate(bool),
    StopNomination(),
    UpdateLocalQuorumSet(),
    ReceivedEnvelope(),
}

#[async_trait]
impl proto::SCP_Control_SCP_Control1 for ControlService {
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

use crate::handler::*;

#[derive(Clone)]
pub struct ControlKernel(Arc<StdRwLock<ControlKernelState>>);

struct ControlKernelState {
    rx_request: Receiver<ControlRequest>,
    tx_reply: Sender<ControlReply>,
    kernel: StellarKernel<HandleKernel, HandleKernel>,
    handler: HandleKernel,
    rx_switch_timer: UnboundedReceiver<SwitchTimer>,
    armed_timers: HashMap<SlotIndex, HashMap<Timer, oneshot::Sender<()>>>,
    tx_validate_envelope: Sender<ValidateEnvelope>,
    rx_envelope_validated: Receiver<EnvelopeValidated>,
}

pub type ValidatedValues = HashMap<SlotIndex, HashMap<SCPValue, ValidationLevel>>;

pub struct ValidateEnvelope(SCPEnvelope);
pub struct EnvelopeValidated(SCPEnvelope, ValidatedValues);

impl ControlKernel {
    pub fn new(
        rx_request: Receiver<ControlRequest>,
        tx_reply: Sender<ControlReply>,
        rx_switch_timer: UnboundedReceiver<SwitchTimer>,
        tx_validate_envelope: Sender<ValidateEnvelope>,
        rx_envelope_validated: Receiver<EnvelopeValidated>,
        kernel: StellarKernel<HandleKernel, HandleKernel>,
        handler: HandleKernel,
    ) -> ControlKernel {
        ControlKernel(Arc::new(StdRwLock::new(ControlKernelState {
            rx_request,
            tx_reply,
            kernel,
            handler,
            rx_switch_timer,
            armed_timers: HashMap::new(),
            tx_validate_envelope,
            rx_envelope_validated,
        })))
    }

    fn lock(&self) -> StdRwLockReadGuard<'_, ControlKernelState> {
        self.0.read().unwrap()
    }

    fn lock_mut(&self) -> StdRwLockWriteGuard<'_, ControlKernelState> {
        self.0.write().unwrap()
    }

    fn flush_timer_switches(&self) {
        loop {
            if let Some(v) = match self.lock_mut().rx_switch_timer.try_recv() {
                Ok(v) => Some(v),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => panic!("bug"),
            } {
                match v {
                    SwitchTimer::StartTimer(slot_index, timer, timeout, cb) => {
                        self.start_timer(slot_index, timer, timeout, cb);
                    }
                    SwitchTimer::StopTimer(slot_index, timer) => {
                        self.stop_timer(slot_index, timer);
                    }
                }
            } else {
                break;
            }
        }
    }

    fn start_timer(
        &self,
        slot_index: SlotIndex,
        timer: Timer,
        timeout: Duration,
        cb: DriverCallback,
    ) {
        self.stop_timer(slot_index, timer);

        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        tokio::spawn({
            let x = self.clone();
            async move {
                tokio::select! {
                    _ = sleep(timeout) => {
                        tokio::task::spawn_blocking(move || {
                            x.stop_timer(slot_index, timer);
                            println!("firing timeout {:?}", cb);
                            x.lock_mut().kernel.proto().driver_calling_back(cb);
                            x.flush_timer_switches();
                        });
                    }
                    _ = cancel_rx => {
                        println!("timeout cancelled {:?}", cb);
                    }
                }
            }
        });

        let mut lock = self.lock_mut();
        let slot_timers = lock
            .armed_timers
            .entry(slot_index)
            .or_insert_with(HashMap::new);
        if let Some(_) = slot_timers.insert(timer, cancel_tx) {
            panic!("there should be no prior timer")
        }
    }

    fn stop_timer(&self, slot_index: SlotIndex, timer: Timer) {
        let mut lock = self.lock_mut();
        if let Some(slot_timers) = lock.armed_timers.get_mut(&slot_index) {
            if let Some(cancel_tx) = slot_timers.remove(&timer) {
                let _ = cancel_tx.send(());
            }
        }
    }

    pub fn start(&self) -> std::thread::JoinHandle<()> {
        let x = self.clone();
        thread::spawn(move || {
            loop {
                let req = x.lock_mut().rx_request.blocking_recv().unwrap();
                match req {
                    ControlRequest::Nominate(slot_index, value, prev_value) => {
                        let r =
                            x.lock_mut()
                                .kernel
                                .proto()
                                .nominate(slot_index, &value, &prev_value);
                        x.flush_timer_switches();
                        x.lock()
                            .tx_reply
                            .blocking_send(ControlReply::Nominate(r))
                            .unwrap()
                    }
                    ControlRequest::StopNomination(slot_index) => {
                        x.lock_mut().kernel.proto().stop_nomination(slot_index);
                        x.flush_timer_switches();
                        x.lock()
                            .tx_reply
                            .blocking_send(ControlReply::StopNomination())
                            .unwrap()
                    }
                    ControlRequest::UpdateLocalQuorumSet(qset) => {
                        x.lock_mut().kernel.proto().update_local_quorum_set(&qset);
                        x.flush_timer_switches();
                        x.lock()
                            .tx_reply
                            .blocking_send(ControlReply::UpdateLocalQuorumSet())
                            .unwrap()
                    }
                    ControlRequest::ReceivedEnvelope(env) => {
                        // XXX: ask kernel if the envelope should be validated
                        x.lock()
                            .tx_validate_envelope
                            .blocking_send(ValidateEnvelope(env))
                            .unwrap();

                        // x.lock_mut().kernel.proto().received_envelope(&env);
                        // x.flush_timer_switches();
                        // x.lock()
                        //     .tx_reply
                        //     .blocking_send(ControlReply::ReceivedEnvelope())
                        //     .unwrap()
                    }
                }
            }
        })
    }
}
