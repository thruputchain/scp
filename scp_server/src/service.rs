use async_trait::async_trait;
use scp_kernel::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;

use std::sync::RwLock as StdRwLock;
use std::sync::RwLockReadGuard as StdRwLockReadGuard;
use std::sync::RwLockWriteGuard as StdRwLockWriteGuard;

use tokio::sync::RwLock as TokioRwLock;
use tokio::sync::RwLockReadGuard as TokioRwLockReadGuard;
use tokio::sync::RwLockWriteGuard as TokioRwLockWriteGuard;

use tokio::time::sleep;
use xdr_common::*;

#[derive(Clone)]
pub struct Service(Arc<TokioRwLock<Kernel>>);

impl Service {
    pub fn new(
        handle: SCP_Handle_SCP_Handle1_Client,
        cfg: proto::SCPConfig,
        secret_key: &proto::SecretKey,
        is_validator: bool,
    ) -> Service {
        Service(Arc::new(TokioRwLock::new(Kernel::new(
            handle,
            cfg,
            secret_key,
            is_validator,
        ))))
    }

    async fn lock(&self) -> TokioRwLockReadGuard<'_, Kernel> {
        self.0.read().await
    }

    async fn lock_mut(&self) -> TokioRwLockWriteGuard<'_, Kernel> {
        self.0.write().await
    }

    fn start_timer(
        &self,
        slot_index: SlotIndex,
        timer: Timer,
        timeout: Duration,
        cb: DriverCallback,
    ) {
        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();

        tokio::spawn({
            let x = self.clone();
            async move {
                tokio::select! {
                    _ = sleep(timeout) => {
                        x.lock_mut().await.kernel.proto().driver_calling_back(cb);
                    }
                    _ = cancel_rx => {
                    }
                }
            }
        });

        todo!("XXX")
    }

    fn stop_timer(&self, slot_index: SlotIndex, timer: Timer) {
        todo!()
    }

    async fn flush_kernel_calls(&self) {
        let kernel = self.lock_mut().await;
        // let mut state = kernel.state.lock_mut();

        let mut emitted_envelopes: Vec<SCPEnvelope> = kernel
            .state
            .lock_mut()
            .emitted_envelopes
            .drain(..)
            .collect();

        let mut switched_timers: Vec<TimerSwitch> =
            kernel.state.lock_mut().switched_timers.drain(..).collect();

        let mut issued_notifications: Vec<Notification> = kernel
            .state
            .lock_mut()
            .issued_notifications
            .drain(..)
            .collect();

        // flush emitted envelopes
        for env in emitted_envelopes.drain(..) {
            kernel.handle.scp_handle_emit_envelope(env).await.unwrap();
        }

        // flush timer switches
        for switch in switched_timers.drain(..) {
            match switch {
                TimerSwitch::Start(slot_index, timer, timeout, cb) => {
                    self.start_timer(slot_index, timer, timeout, cb);
                }
                TimerSwitch::Stop(slot_index, timer) => {
                    self.stop_timer(slot_index, timer);
                }
            }
        }

        // flush notifications
        for notification in issued_notifications.drain(..) {
            match notification {
                Notification::ValueExternalized(slot_index, value) => {
                    kernel
                        .handle
                        .scp_handle_value_externalized(slot_index, value)
                        .await
                        .unwrap();
                }
                Notification::StartedBallotProto(slot_index, ballot) => {
                    kernel
                        .handle
                        .scp_handle_started_ballot_proto(slot_index, ballot)
                        .await
                        .unwrap();
                }
                Notification::AcceptedBallotPrepared(slot_index, ballot) => {
                    kernel
                        .handle
                        .scp_handle_accepted_ballot_prepared(slot_index, ballot)
                        .await
                        .unwrap();
                }
                Notification::ConfirmedBallotPrepared(slot_index, ballot) => {
                    kernel
                        .handle
                        .scp_handle_confirmed_ballot_prepared(slot_index, ballot)
                        .await
                        .unwrap();
                }
                Notification::AcceptedCommit(slot_index, ballot) => {
                    kernel
                        .handle
                        .scp_handle_accepted_commit(slot_index, ballot)
                        .await
                        .unwrap();
                }
                Notification::BallotDidHearFromQuorum(slot_index, ballot) => {
                    kernel
                        .handle
                        .scp_handle_ballot_did_hear_from_quorum(slot_index, ballot)
                        .await
                        .unwrap();
                }
                Notification::NominatingValue(slot_index, value) => {
                    kernel
                        .handle
                        .scp_handle_nominating_value(slot_index, value)
                        .await
                        .unwrap();
                }
                Notification::UpdatedCandidateValue(slot_index, value) => {
                    kernel
                        .handle
                        .scp_handle_updated_candidate_value(slot_index, value)
                        .await
                        .unwrap();
                }
            }
        }
    }
}

#[async_trait]
impl proto::SCP_Control_SCP_Control1 for Service {
    async fn scp_control_null(&self) -> Result<(), ::rpc_driver::ProgramError> {
        Ok(())
    }

    async fn scp_control_nominate(
        &self,
        slot_index: SCPSlotIndex,
        value: SCPValue,
        prev_value: SCPValue,
    ) -> Result<bool, ::rpc_driver::ProgramError> {
        let r = self
            .lock_mut()
            .await
            .kernel
            .proto()
            .nominate(slot_index, &value, &prev_value);
        self.flush_kernel_calls().await;
        Ok(r)
    }

    async fn scp_control_stop_nomination(
        &self,
        slot_index: SCPSlotIndex,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        self.lock_mut()
            .await
            .kernel
            .proto()
            .stop_nomination(slot_index);
        self.flush_kernel_calls().await;
        Ok(())
    }

    async fn scp_control_update_local_quorum_set(
        &self,
        qset: SCPQuorumSet,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        self.lock_mut()
            .await
            .kernel
            .proto()
            .update_local_quorum_set(&qset);
        self.flush_kernel_calls().await;
        Ok(())
    }

    async fn scp_control_received_envelope(
        &self,
        env: SCPEnvelope,
    ) -> Result<(), ::rpc_driver::ProgramError> {
        self.lock_mut().await.kernel.proto().received_envelope(&env);
        self.flush_kernel_calls().await;
        Ok(())
    }
}

pub struct Kernel {
    handle: SCP_Handle_SCP_Handle1_Client,
    state: SharedState,
    kernel: StellarKernel<SharedState, SharedState>,
}

impl Kernel {
    fn new(
        handle: SCP_Handle_SCP_Handle1_Client,
        cfg: proto::SCPConfig,
        secret_key: &proto::SecretKey,
        is_validator: bool,
    ) -> Kernel {
        let state = SharedState::new(&cfg, &secret_key);
        Kernel {
            handle: handle,
            state: state.clone(),
            kernel: StellarKernel::new(
                derive_node_id(&secret_key),
                is_validator,
                cfg.quorum_set,
                state.clone(),
                state.clone(),
            ),
        }
    }
}

#[derive(Debug)]
pub struct State {
    max_timeout: Duration,
    initial_ballot_timeout: Duration,
    increment_ballot_timeout: Duration,
    initial_nomination_timeout: Duration,
    increment_nomination_timeout: Duration,
    //
    secret_key: proto::SecretKey,
    quorum_sets: HashMap<proto::SCPHash, proto::SCPQuorumSet>,
    //
    emitted_envelopes: VecDeque<proto::SCPEnvelope>,
    switched_timers: VecDeque<TimerSwitch>,
    issued_notifications: VecDeque<Notification>,
    //
    armed_timers: HashMap<SlotIndex, HashMap<Timer, oneshot::Sender<()>>>,
}

#[derive(Debug)]
enum TimerSwitch {
    Start(SlotIndex, Timer, Duration, DriverCallback),
    Stop(SlotIndex, Timer),
}

impl State {
    pub fn new(cfg: &proto::SCPConfig, secret_key: &proto::SecretKey) -> State {
        State {
            max_timeout: Duration::from_millis(cfg.max_timeout_ms),
            initial_ballot_timeout: Duration::from_millis(cfg.initial_ballot_timeout_ms),
            increment_ballot_timeout: Duration::from_millis(cfg.increment_ballot_timeout_ms),
            initial_nomination_timeout: Duration::from_millis(cfg.initial_nomination_timeout_ms),
            increment_nomination_timeout: Duration::from_millis(
                cfg.increment_nomination_timeout_ms,
            ),
            secret_key: secret_key.clone(),
            quorum_sets: HashMap::new(),
            emitted_envelopes: VecDeque::new(),
            switched_timers: VecDeque::new(),
            issued_notifications: VecDeque::new(),
            armed_timers: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SharedState(pub Arc<StdRwLock<State>>);

impl SharedState {
    pub fn new(cfg: &proto::SCPConfig, secret_key: &proto::SecretKey) -> SharedState {
        SharedState(Arc::new(StdRwLock::new(State::new(cfg, secret_key))))
    }

    fn lock(&self) -> StdRwLockReadGuard<'_, State> {
        self.0.read().unwrap()
    }

    fn lock_mut(&self) -> StdRwLockWriteGuard<'_, State> {
        self.0.write().unwrap()
    }
}

impl Consensus for SharedState {
    fn validate_ballot_value(
        &self,
        slot_index: SCPSlotIndex,
        value: &proto::SCPValue,
    ) -> ValidationLevel {
        todo!()
    }

    fn validate_nomination_value(
        &self,
        slot_index: SCPSlotIndex,
        value: &proto::SCPValue,
    ) -> ValidationLevel {
        todo!()
    }

    fn extract_valid_value(
        &self,
        slot_index: SCPSlotIndex,
        value: &proto::SCPValue,
    ) -> Option<proto::SCPValue> {
        todo!()
    }

    fn combine_candidates(
        &self,
        slot_index: SCPSlotIndex,
        candidates: &HashSet<proto::SCPValue>,
    ) -> Option<proto::SCPValue> {
        todo!()
    }
}

#[derive(Debug)]
enum Notification {
    ValueExternalized(SlotIndex, SCPValue),
    StartedBallotProto(SlotIndex, SCPBallot),
    AcceptedBallotPrepared(SlotIndex, SCPBallot),
    ConfirmedBallotPrepared(SlotIndex, SCPBallot),
    AcceptedCommit(SlotIndex, SCPBallot),
    BallotDidHearFromQuorum(SlotIndex, SCPBallot),
    NominatingValue(SlotIndex, SCPValue),
    UpdatedCandidateValue(SlotIndex, SCPValue),
}

impl ConsensusNotify for SharedState {
    fn value_externalized(&self, slot_index: SlotIndex, value: &proto::SCPValue) {
        self.lock_mut()
            .issued_notifications
            .push_back(Notification::ValueExternalized(slot_index, value.clone()));
    }

    fn started_ballot_proto(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        self.lock_mut()
            .issued_notifications
            .push_back(Notification::StartedBallotProto(slot_index, ballot.clone()));
    }

    fn accepted_ballot_prepared(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        self.lock_mut()
            .issued_notifications
            .push_back(Notification::AcceptedBallotPrepared(
                slot_index,
                ballot.clone(),
            ));
    }

    fn confirmed_ballot_prepared(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        self.lock_mut()
            .issued_notifications
            .push_back(Notification::ConfirmedBallotPrepared(
                slot_index,
                ballot.clone(),
            ));
    }

    fn accepted_commit(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        self.lock_mut()
            .issued_notifications
            .push_back(Notification::AcceptedCommit(slot_index, ballot.clone()));
    }

    fn ballot_did_hear_from_quorum(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        self.lock_mut()
            .issued_notifications
            .push_back(Notification::BallotDidHearFromQuorum(
                slot_index,
                ballot.clone(),
            ));
    }

    fn nominating_value(&self, slot_index: SlotIndex, value: &proto::SCPValue) {
        self.lock_mut()
            .issued_notifications
            .push_back(Notification::NominatingValue(slot_index, value.clone()));
    }

    fn updated_candidate_value(&self, slot_index: SlotIndex, value: &proto::SCPValue) {
        self.lock_mut()
            .issued_notifications
            .push_back(Notification::UpdatedCandidateValue(
                slot_index,
                value.clone(),
            ));
    }
}

impl Driver for SharedState {
    fn get_qset(&self, h: &proto::SCPHash) -> Option<proto::SCPQuorumSet> {
        self.lock().quorum_sets.get(h).cloned()
    }

    fn to_short_string(&self, pk: &proto::SCPNodeID) -> String {
        self.to_str_key(pk, false)
    }

    fn to_str_key(&self, pk: &proto::SCPNodeID, full_key: bool) -> String {
        match pk {
            PublicKey::Ed25519 { ed25519 } => {
                if full_key {
                    hex_full(&ed25519.into_inner()[..])
                } else {
                    hex_abbrev(&ed25519.into_inner()[..])
                }
            }
        }
    }

    fn from_str_key(&self, s: &str) -> Result<proto::SCPNodeID, String> {
        parse_node_id_from_hex(s)
    }

    fn get_value_string(&self, value: &proto::SCPValue) -> String {
        format!("{}", hex_abbrev(&value.0))
    }

    fn get_hash_of(&self, opaque: &[u8]) -> proto::SCPHash {
        hash_bytes(opaque)
    }

    fn get_node_weight(
        &self,
        node_id: &proto::SCPNodeID,
        qset: &proto::SCPQuorumSet,
        is_local_node: bool,
    ) -> u64 {
        get_node_weight(node_id, qset, is_local_node)
    }

    fn compute_hash_node(
        &self,
        slot_index: proto::SCPSlotIndex,
        prev: &proto::SCPValue,
        is_priority: bool,
        round_number: proto::SCPCount,
        node_id: &proto::SCPNodeID,
    ) -> u64 {
        hash_helper(
            slot_index,
            prev,
            |v| {
                v.push(BytesM(
                    proto::marshal_xdr(if is_priority { &HASH_P } else { &HASH_N })
                        .unwrap()
                        .to_vec(),
                ));
                v.push(BytesM(proto::marshal_xdr(&round_number).unwrap().to_vec()));
                v.push(BytesM(proto::marshal_xdr(node_id).unwrap().to_vec()));
            },
            |b| self.get_hash_of(b),
        )
    }

    fn compute_value_hash(
        &self,
        slot_index: proto::SCPSlotIndex,
        prev: &proto::SCPValue,
        round_number: proto::SCPCount,
        value: &proto::SCPValue,
    ) -> u64 {
        hash_helper(
            slot_index,
            prev,
            |v| {
                v.push(BytesM(proto::marshal_xdr(&HASH_K).unwrap().to_vec()));
                v.push(BytesM(proto::marshal_xdr(&round_number).unwrap().to_vec()));
                v.push(BytesM(proto::marshal_xdr(value).unwrap().to_vec()));
            },
            |b| self.get_hash_of(b),
        )
    }

    fn sign_envelope(&self, env: &mut proto::SCPEnvelope) {
        env.signature = sign_xdr(&self.lock().secret_key, &env.statement).unwrap()
    }

    fn compute_timeout(&self, round_number: proto::SCPCount, is_nomination: bool) -> Duration {
        let (initial_timeout_ms, increment_ms) = if is_nomination {
            (
                self.lock().initial_nomination_timeout.as_millis(),
                self.lock().increment_nomination_timeout.as_millis(),
            )
        } else {
            (
                self.lock().initial_ballot_timeout.as_millis(),
                self.lock().increment_ballot_timeout.as_millis(),
            )
        };

        let timeout_ms =
            initial_timeout_ms as u64 + (round_number as u64 - 1) * increment_ms as u64;
        let timeout_ms = std::cmp::min(timeout_ms, self.lock().max_timeout.as_millis() as u64);

        Duration::from_millis(timeout_ms as u64)
    }

    fn emit_envelope(&self, env: proto::SCPEnvelope) {
        self.lock_mut().emitted_envelopes.push_back(env);
    }

    fn start_timer(
        &self,
        slot_index: SlotIndex,
        timer: Timer,
        timeout: Duration,
        cb: DriverCallback,
    ) {
        self.lock_mut()
            .switched_timers
            .push_back(TimerSwitch::Start(slot_index, timer, timeout, cb));
    }

    fn stop_timer(&self, slot_index: SlotIndex, timer: Timer) {
        self.lock_mut()
            .switched_timers
            .push_back(TimerSwitch::Stop(slot_index, timer));
    }
}
