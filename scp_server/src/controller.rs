use async_trait::async_trait;
use scp_kernel::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::RwLock as TokioRwLock;
use tokio::sync::RwLockWriteGuard as TokioRwLockWriteGuard;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender, UnboundedReceiver, error::TryRecvError};
use tokio::sync::oneshot;
use tokio::time::sleep;
use tracing::info;
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

pub type ValidatedValues = HashMap<SlotIndex, HashMap<SCPValue, ValidationLevel>>;

pub struct ValidateEnvelope {
    pub env: SCPEnvelope,
    pub validate_nomination_values: Vec<SCPValue>,
    pub validate_ballot_values: Vec<SCPValue>,
}

pub struct EnvelopeValidated {
    pub env: SCPEnvelope,
    pub validated_values: Vec<(SlotIndex, SCPValue, ValidationLevel)>,
}

#[derive(Debug)]
pub struct TimerExpired {
    slot_index: SlotIndex,
    timer: Timer,
    cb: DriverCallback,
}

pub enum ControlSignal {
    ControlRequest(ControlRequest),
    EnvelopeValidated(EnvelopeValidated),
    TimerExpired(TimerExpired),
}

pub struct ControlKernel {
    tokio_handle: tokio::runtime::Handle,
    rx_signal: Receiver<ControlSignal>,
    tx_reply: Sender<ControlReply>,
    kernel: StellarKernel<HandleKernel, HandleKernel>,
    handler: HandleKernel,
    rx_switch_timer: UnboundedReceiver<SwitchTimer>,
    armed_timers: HashMap<SlotIndex, HashMap<Timer, oneshot::Sender<()>>>,
    tx_validate_envelope: Sender<ValidateEnvelope>,
    tx_timer_expired: Arc<TokioRwLock<Sender<TimerExpired>>>,
}

impl ControlKernel {
    pub async fn new(
        tokio_handle: tokio::runtime::Handle,
        rx_request: Receiver<ControlRequest>,
        tx_reply: Sender<ControlReply>,
        rx_switch_timer: UnboundedReceiver<SwitchTimer>,
        tx_validate_envelope: Sender<ValidateEnvelope>,
        rx_envelope_validated: Receiver<EnvelopeValidated>,
        kernel: StellarKernel<HandleKernel, HandleKernel>,
        handler: HandleKernel,
    ) -> ControlKernel {
        let (tx_timer_expired, rx_timer_expired) = mpsc::channel::<TimerExpired>(1);
        let rx_request_or_validated =
            Self::merge_rx(rx_request, rx_envelope_validated, rx_timer_expired).await;
        ControlKernel {
            tokio_handle,
            rx_signal: rx_request_or_validated,
            tx_reply,
            kernel,
            handler,
            rx_switch_timer,
            armed_timers: HashMap::new(),
            tx_validate_envelope,
            tx_timer_expired: Arc::new(TokioRwLock::new(tx_timer_expired)),
        }
    }

    pub async fn merge_rx(
        mut rx_request: Receiver<ControlRequest>,
        mut rx_envelope_validated: Receiver<EnvelopeValidated>,
        mut rx_timer_expired: Receiver<TimerExpired>,
    ) -> Receiver<ControlSignal> {
        let (tx, rx) = mpsc::channel::<ControlSignal>(1);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = rx_request.recv() => {
                        tx.send(ControlSignal::ControlRequest(msg.unwrap())).await.unwrap();
                    },
                    msg = rx_envelope_validated.recv() => {
                        tx.send(ControlSignal::EnvelopeValidated(msg.unwrap())).await.unwrap();
                    }
                    msg = rx_timer_expired.recv() => {
                        tx.send(ControlSignal::TimerExpired(msg.unwrap())).await.unwrap();
                    }
                }
            }
        });
        rx
    }

    fn flush_timer_switches(&mut self) {
        loop {
            if let Some(v) = match self.rx_switch_timer.try_recv() {
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
        &mut self,
        slot_index: SlotIndex,
        timer: Timer,
        timeout: Duration,
        cb: DriverCallback,
    ) {
        self.stop_timer(slot_index, timer);

        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();

        self.tokio_handle.spawn({
            let tx_timer_expired = self.tx_timer_expired.clone();
            async move {
                tokio::select! {
                    _ = sleep(timeout) => {
                        tx_timer_expired.write().await.send(TimerExpired{slot_index, timer, cb}).await.unwrap();
                    }
                    _ = cancel_rx => {
                    }
                }
            }
        });

        let slot_timers = self
            .armed_timers
            .entry(slot_index)
            .or_insert_with(HashMap::new);
        if let Some(_) = slot_timers.insert(timer, cancel_tx) {
            panic!("there should be no prior timer")
        }
    }

    fn stop_timer(&mut self, slot_index: SlotIndex, timer: Timer) {
        if let Some(slot_timers) = self.armed_timers.get_mut(&slot_index) {
            if let Some(cancel_tx) = slot_timers.remove(&timer) {
                let _ = cancel_tx.send(());
            }
        }
    }

    pub fn start(mut self) -> std::thread::JoinHandle<()> {
        thread::spawn(move || {
            loop {
                let msg = self.rx_signal.blocking_recv().unwrap();
                match msg {
                    ControlSignal::TimerExpired(x) => {
                        self.handle_timer_expired(x);
                    }
                    ControlSignal::ControlRequest(x) => {
                        self.handle_request(x);
                    }
                    ControlSignal::EnvelopeValidated(x) => {
                        self.handle_envelope_validated(x);
                    }
                }
            }
        })
    }

    fn handle_envelope_validated(&mut self, x: EnvelopeValidated) {
        for (slot_index, value, level) in x.validated_values.into_iter() {
            self.handler.set_cached_validation(slot_index, value, level);
        }
        self.kernel.proto().received_envelope(&x.env);
        self.flush_timer_switches();
    }

    fn handle_timer_expired(&mut self, x: TimerExpired) {
        info!("timer expired {:?}", x);
        self.stop_timer(x.slot_index, x.timer);
        self.kernel.proto().driver_calling_back(x.cb);
        self.flush_timer_switches();
    }

    fn handle_request(&mut self, req: ControlRequest) {
        match req {
            ControlRequest::Nominate(slot_index, value, prev_value) => {
                // pre-register our nominations as fully valid;
                // necessary because `nominate` calls `process_envelope` on its own envelopes internally
                self.handler.set_cached_validation(
                    slot_index,
                    value.clone(),
                    ValidationLevel::FullyValid,
                );
                let r = self
                    .kernel
                    .proto()
                    .nominate(slot_index, &value, &prev_value);
                self.flush_timer_switches();
                self.tx_reply
                    .blocking_send(ControlReply::Nominate(r))
                    .unwrap()
            }
            ControlRequest::StopNomination(slot_index) => {
                self.kernel.proto().stop_nomination(slot_index);
                self.flush_timer_switches();
                self.tx_reply
                    .blocking_send(ControlReply::StopNomination())
                    .unwrap()
            }
            ControlRequest::UpdateLocalQuorumSet(qset) => {
                self.kernel.proto().update_local_quorum_set(&qset);
                self.flush_timer_switches();
                self.tx_reply
                    .blocking_send(ControlReply::UpdateLocalQuorumSet())
                    .unwrap()
            }
            ControlRequest::ReceivedEnvelope(env) => {
                if self.kernel.proto().should_process_envelope(&env) {
                    let slot_index = env.statement.slot_index;
                    let (nom_values, ballot_values) =
                        StellarProto::<'_, HandleKernel, HandleKernel>::get_statement_values(
                            &env.statement,
                        );

                    // filter non-cached values
                    let validate_nomination_values = nom_values
                        .into_iter()
                        .filter(|value| {
                            self.handler
                                .get_cached_validation(slot_index, value)
                                .is_none()
                        })
                        .collect::<Vec<_>>();
                    let validate_ballot_values = ballot_values
                        .into_iter()
                        .filter(|value| {
                            self.handler
                                .get_cached_validation(slot_index, value)
                                .is_none()
                        })
                        .collect::<Vec<_>>();

                    // skip validate_envelope if no values to validate
                    if validate_nomination_values.is_empty() && validate_ballot_values.is_empty() {
                        self.kernel.proto().received_envelope(&env);
                        self.flush_timer_switches();
                    } else {
                        self.tx_validate_envelope
                            .blocking_send(ValidateEnvelope {
                                env,
                                validate_nomination_values,
                                validate_ballot_values,
                            })
                            .unwrap();
                    }
                    self.tx_reply
                        .blocking_send(ControlReply::ReceivedEnvelope())
                        .unwrap()
                }
            }
        }
    }
}
