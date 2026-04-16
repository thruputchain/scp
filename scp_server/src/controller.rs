use async_trait::async_trait;
use scp_kernel::*;
use scp_proto::*;
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

#[derive(Clone)]
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

    pub async fn control_set_qset(
        &self,
        qset: SCPQuorumSet,
    ) -> Result<SCPOptQuorumSet, ::rpc_driver::ProgramError> {
        let mut x = self.lock_mut().await;
        x.tx_request
            .send(ControlRequest::SetKnownQSet(qset))
            .await
            .unwrap();
        if let ControlReply::SetKnownQSet(optq) = x.rx_reply.recv().await.unwrap() {
            Ok(optq)
        } else {
            panic!("bug")
        }
    }

    pub async fn control_nominate(
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

    pub async fn control_received_envelope(
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

#[allow(clippy::large_enum_variant)]
pub enum ControlRequest {
    Nominate(SCPSlotIndex, SCPValue, SCPValue),
    StopNomination(SCPSlotIndex),
    UpdateLocalQuorumSet(SCPQuorumSet),
    ReceivedEnvelope(SCPEnvelope),
    SetKnownQSet(SCPQuorumSet),
    GetKnownQSet(SCPHash),
    RemoveKnownQSet(SCPHash),
    SetLowestNonExternalizedSlot(SCPSlotIndex),
}

pub enum ControlReply {
    Nominate(bool),
    StopNomination(),
    UpdateLocalQuorumSet(),
    ReceivedEnvelope(),
    SetKnownQSet(Option<SCPQuorumSet>),
    GetKnownQSet(Option<SCPQuorumSet>),
    RemoveKnownQSet(Option<SCPQuorumSet>),
    SetLowestNonExternalizedSlot(),
}

#[async_trait]
impl scp_proto::SCP_Control_SCP_Control1 for ControlService {
    async fn scp_control_null(&self) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_control_nominate(
        &self,
        slot_index: SCPSlotIndex,
        value: SCPValue,
        prev_value: SCPValue,
    ) -> Result<bool, ::rpc_driver::ProgramError> {
        self.control_nominate(slot_index, value, prev_value).await
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

    #[tracing::instrument(
        skip(self, env),
        fields(slot_index = env.statement.slot_index)
    )]
    async fn scp_control_received_envelope(
        &self,
        env: SCPEnvelope,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        self.control_received_envelope(env).await
    }

    async fn scp_control_set_qset(
        &self,
        qset: SCPQuorumSet,
    ) -> Result<SCPOptQuorumSet, ::rpc_driver::ProgramError> {
        self.control_set_qset(qset).await
    }

    async fn scp_control_get_qset(
        &self,
        qhash: SCPHash,
    ) -> Result<SCPOptQuorumSet, ::rpc_driver::ProgramError> {
        let mut x = self.lock_mut().await;
        x.tx_request
            .send(ControlRequest::GetKnownQSet(qhash))
            .await
            .unwrap();
        if let ControlReply::GetKnownQSet(optq) = x.rx_reply.recv().await.unwrap() {
            Ok(optq)
        } else {
            panic!("bug")
        }
    }

    async fn scp_control_remove_qset(
        &self,
        qhash: SCPHash,
    ) -> Result<SCPOptQuorumSet, ::rpc_driver::ProgramError> {
        let mut x = self.lock_mut().await;
        x.tx_request
            .send(ControlRequest::RemoveKnownQSet(qhash))
            .await
            .unwrap();
        if let ControlReply::RemoveKnownQSet(optq) = x.rx_reply.recv().await.unwrap() {
            Ok(optq)
        } else {
            panic!("bug")
        }
    }

    async fn scp_control_set_lowest_non_externalized_slot(
        &self,
        slot_index: SCPSlotIndex,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        let mut x = self.lock_mut().await;
        x.tx_request
            .send(ControlRequest::SetLowestNonExternalizedSlot(slot_index))
            .await
            .unwrap();
        if let ControlReply::SetLowestNonExternalizedSlot() = x.rx_reply.recv().await.unwrap() {
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

#[derive(Debug, Clone)]
pub struct TimerExpired {
    slot_index: SlotIndex,
    timer: Timer,
    cb: DriverCallback,
}

impl std::fmt::Display for TimerExpired {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TimerExpired{{ i: {} | {} }}", self.slot_index, self.cb,)
    }
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
    #[allow(clippy::too_many_arguments)]
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

    #[tracing::instrument(skip_all)]
    fn flush_timer_switches(&mut self) {
        loop {
            match self.rx_switch_timer.try_recv() {
                Ok(v) => match v {
                    SwitchTimer::StartTimer(slot_index, timer, timeout, cb) => {
                        self.start_timer(slot_index, timer, timeout, cb);
                    }
                    SwitchTimer::StopTimer(slot_index, timer) => {
                        self.stop_timer(slot_index, timer);
                    }
                    SwitchTimer::SlotExternalized(slot_index) => {
                        self.stop_timers_not_above(slot_index);
                        // purge slots not above slot_index
                        self.kernel
                            .proto()
                            .purge_slots(slot_index + 1, slot_index + 1);
                    }
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("bug"),
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

        let slot_timers = self.armed_timers.entry(slot_index).or_default();
        if slot_timers.insert(timer, cancel_tx).is_some() {
            panic!("there should be no prior timer")
        }
    }

    fn stop_timer(&mut self, slot_index: SlotIndex, timer: Timer) {
        if let Some(slot_timers) = self.armed_timers.get_mut(&slot_index)
            && let Some(cancel_tx) = slot_timers.remove(&timer)
        {
            let _ = cancel_tx.send(());
        }
    }

    fn stop_timers_not_above(&mut self, not_above: SlotIndex) {
        let mut to_stop = vec![];
        for (slot_index, timers) in &self.armed_timers {
            if *slot_index <= not_above {
                for timer in timers.keys() {
                    to_stop.push((*slot_index, *timer));
                }
            }
        }
        for (slot_index, timer) in to_stop {
            self.stop_timer(slot_index, timer);
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
        self.call_received_envelope(&x.env);
    }

    fn call_received_envelope(&mut self, env: &scp_proto::SCPEnvelope) {
        info!(
            "---> control kernel calling scp received envelope {}",
            self.kernel.proto().envelope_to_str(env, false),
        );
        if self
            .handler
            .is_above_highest_externalized(env.statement.slot_index)
        {
            self.kernel.proto().received_envelope(env);
            self.flush_timer_switches();
        }
        info!(
            "<--- control kernel called scp receive envelope {}",
            self.kernel.proto().envelope_to_str(env, false),
        );
    }

    #[tracing::instrument(skip_all)]
    fn handle_timer_expired(&mut self, x: TimerExpired) {
        info!("---> timer expired, driver calling back {}", x);
        self.stop_timer(x.slot_index, x.timer);
        self.kernel.proto().driver_calling_back(x.cb.clone());
        self.flush_timer_switches();
        info!("<--- timer expired, driver called back {}", x);
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
                info!(
                    "control kernel received envelope {}",
                    self.kernel.proto().envelope_to_str(&env, false)
                );
                if self
                    .handler
                    .is_above_highest_externalized(env.statement.slot_index)
                    && self.kernel.proto().should_process_envelope(&env)
                {
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
                        self.call_received_envelope(&env);
                    } else {
                        self.tx_validate_envelope
                            .blocking_send(ValidateEnvelope {
                                env,
                                validate_nomination_values,
                                validate_ballot_values,
                            })
                            .unwrap();
                    }
                }
                self.tx_reply
                    .blocking_send(ControlReply::ReceivedEnvelope())
                    .unwrap()
            }
            ControlRequest::SetKnownQSet(qset) => {
                let optq = self.handler.set_known_qset(qset);
                self.tx_reply
                    .blocking_send(ControlReply::SetKnownQSet(optq))
                    .unwrap()
            }
            ControlRequest::GetKnownQSet(qhash) => {
                let optq = self.handler.get_known_qset(&qhash);
                self.tx_reply
                    .blocking_send(ControlReply::GetKnownQSet(optq))
                    .unwrap()
            }
            ControlRequest::RemoveKnownQSet(qhash) => {
                let optq = self.handler.remove_known_qset(&qhash);
                self.tx_reply
                    .blocking_send(ControlReply::RemoveKnownQSet(optq))
                    .unwrap()
            }
            ControlRequest::SetLowestNonExternalizedSlot(slot_index) => {
                self.handler.set_lowest_non_externalized_slot(slot_index);
                self.tx_reply
                    .blocking_send(ControlReply::SetLowestNonExternalizedSlot())
                    .unwrap()
            }
        }
    }
}
