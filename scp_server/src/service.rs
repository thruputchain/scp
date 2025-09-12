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

    async fn lock_mut(&self) -> TokioRwLockWriteGuard<'_, Kernel> {
        self.0.write().await
    }

    fn start_timer(
        &self,
        kernel: &TokioRwLockWriteGuard<'_, Kernel>,
        slot_index: SlotIndex,
        timer: Timer,
        timeout: Duration,
        cb: DriverCallback,
    ) {
        self.stop_timer(kernel, slot_index, timer);

        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        tokio::spawn({
            let x = self.clone();
            async move {
                tokio::select! {
                    _ = sleep(timeout) => {
                        let mut kernel = x.lock_mut().await;
                        x.stop_timer(&kernel, slot_index, timer);
                        println!("firing timeout {:?}", cb);
                        kernel.kernel.proto().driver_calling_back(cb.clone());
                        drop(kernel);
                        x.flush_kernel_calls().await;
                    }
                    _ = cancel_rx => {
                        println!("timeout cancelled {:?}", cb);
                    }
                }
            }
        });

        let mut state = kernel.state.lock_mut();
        let slot_timers = state
            .armed_timers
            .entry(slot_index)
            .or_insert_with(HashMap::new);
        if let Some(_) = slot_timers.insert(timer, cancel_tx) {
            panic!("there should be no prior timer")
        }
    }

    fn stop_timer(
        &self,
        kernel: &TokioRwLockWriteGuard<'_, Kernel>,
        slot_index: SlotIndex,
        timer: Timer,
    ) {
        let mut state = kernel.state.lock_mut();
        if let Some(slot_timers) = state.armed_timers.get_mut(&slot_index) {
            if let Some(cancel_tx) = slot_timers.remove(&timer) {
                let _ = cancel_tx.send(());
            }
        }
    }

    async fn flush_kernel_calls(&self) {
        let kernel = self.lock_mut().await;

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
            kernel.handle.scp_handle_emit_envelope(&env).await.unwrap();
        }

        // flush timer switches
        for switch in switched_timers.drain(..) {
            match switch {
                TimerSwitch::Start(slot_index, timer, timeout, cb) => {
                    self.start_timer(&kernel, slot_index, timer, timeout, cb);
                }
                TimerSwitch::Stop(slot_index, timer) => {
                    self.stop_timer(&kernel, slot_index, timer);
                }
            }
        }

        // flush notifications
        for notification in issued_notifications.drain(..) {
            match notification {
                Notification::ValueExternalized(slot_index, value) => {
                    kernel.state.purge_cached_validations(slot_index);
                    kernel
                        .handle
                        .scp_handle_value_externalized(&slot_index, &value)
                        .await
                        .unwrap();
                }
                Notification::StartedBallotProto(slot_index, ballot) => {
                    kernel
                        .handle
                        .scp_handle_started_ballot_proto(&slot_index, &ballot)
                        .await
                        .unwrap();
                }
                Notification::AcceptedBallotPrepared(slot_index, ballot) => {
                    kernel
                        .handle
                        .scp_handle_accepted_ballot_prepared(&slot_index, &ballot)
                        .await
                        .unwrap();
                }
                Notification::ConfirmedBallotPrepared(slot_index, ballot) => {
                    kernel
                        .handle
                        .scp_handle_confirmed_ballot_prepared(&slot_index, &ballot)
                        .await
                        .unwrap();
                }
                Notification::AcceptedCommit(slot_index, ballot) => {
                    kernel
                        .handle
                        .scp_handle_accepted_commit(&slot_index, &ballot)
                        .await
                        .unwrap();
                }
                Notification::BallotDidHearFromQuorum(slot_index, ballot) => {
                    kernel
                        .handle
                        .scp_handle_ballot_did_hear_from_quorum(&slot_index, &ballot)
                        .await
                        .unwrap();
                }
                Notification::NominatingValue(slot_index, value) => {
                    kernel
                        .handle
                        .scp_handle_nominating_value(&slot_index, &value)
                        .await
                        .unwrap();
                }
                Notification::UpdatedCandidateValue(slot_index, value) => {
                    kernel
                        .handle
                        .scp_handle_updated_candidate_value(&slot_index, &value)
                        .await
                        .unwrap();
                }
            }
        }
    }

    async fn exec_validate_ballot_value(&self, slot_index: SCPSlotIndex, value: &proto::SCPValue) {
        let kernel = self.lock_mut().await;
        let p = kernel
            .handle
            .scp_handle_validate_ballot_value(&slot_index, value)
            .await
            .unwrap();
        kernel
            .state
            .set_cached_validation(slot_index, value.clone(), to_validation_level(p));
    }

    async fn exec_validate_nomination_value(
        &self,
        slot_index: SCPSlotIndex,
        value: &proto::SCPValue,
    ) {
        let kernel = self.lock_mut().await;
        let p = kernel
            .handle
            .scp_handle_validate_nomination_value(&slot_index, value)
            .await
            .unwrap();
        kernel
            .state
            .set_cached_validation(slot_index, value.clone(), to_validation_level(p));
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
        // pre-register our nominations as fully valid;
        // necessary because `nominate` calls `process_envelope` on its own envelopes internally
        self.lock_mut().await.state.set_cached_validation(
            slot_index,
            value.clone(),
            ValidationLevel::FullyValid,
        );
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
        //XXX: dos prevention: verify that values are worth pre-validating

        let x = self.clone();
        tokio::spawn(async move {
            let slot_index = env.statement.slot_index;
            let (nom_values, ballot_values) =
                StellarProto::<'_, SharedState, SharedState>::get_statement_values(&env.statement);

            let kernel = x.lock_mut().await;
            let state = &kernel.state;

            let mut handles = Vec::new();
            for v in nom_values {
                if state.get_cached_validation(slot_index, &v).is_none() {
                    handles.push(tokio::spawn({
                        let svc = x.clone();
                        async move {
                            svc.exec_validate_nomination_value(slot_index, &v).await;
                        }
                    }));
                }
            }
            for v in ballot_values {
                if state.get_cached_validation(slot_index, &v).is_none() {
                    handles.push(tokio::spawn({
                        let svc = x.clone();
                        async move {
                            svc.exec_validate_ballot_value(slot_index, &v).await;
                        }
                    }));
                }
            }
            drop(kernel);
            join_all(handles).await;

            // call received_envelope after validations are computed and cached
            x.lock_mut().await.kernel.proto().received_envelope(&env);
            x.flush_kernel_calls().await;
        });

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
        let state = SharedState::new(handle.clone(), &cfg, &secret_key);
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
    validated_values: HashMap<SlotIndex, HashMap<SCPValue, ValidationLevel>>,
    //
    handle: SCP_Handle_SCP_Handle1_Client,
}

use std::fmt;

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("State")
            .field("max_timeout", &self.max_timeout)
            .field("initial_ballot_timeout", &self.initial_ballot_timeout)
            .field("increment_ballot_timeout", &self.increment_ballot_timeout)
            .field(
                "initial_nomination_timeout",
                &self.initial_nomination_timeout,
            )
            .field(
                "increment_nomination_timeout",
                &self.increment_nomination_timeout,
            )
            .field("secret_key", &self.secret_key)
            .field("quorum_sets", &self.quorum_sets)
            .field("emitted_envelopes", &self.emitted_envelopes)
            .field("switched_timers", &self.switched_timers)
            .field("issued_notifications", &self.issued_notifications)
            .field("armed_timers", &self.armed_timers)
            .finish()
    }
}

#[derive(Debug)]
enum TimerSwitch {
    Start(SlotIndex, Timer, Duration, DriverCallback),
    Stop(SlotIndex, Timer),
}

impl State {
    pub fn new(
        handle: SCP_Handle_SCP_Handle1_Client,
        cfg: &proto::SCPConfig,
        secret_key: &proto::SecretKey,
    ) -> State {
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
            validated_values: HashMap::new(),
            handle,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SharedState(pub Arc<StdRwLock<State>>);

impl SharedState {
    pub fn new(
        handle: SCP_Handle_SCP_Handle1_Client,
        cfg: &proto::SCPConfig,
        secret_key: &proto::SecretKey,
    ) -> SharedState {
        SharedState(Arc::new(StdRwLock::new(State::new(
            handle, cfg, secret_key,
        ))))
    }

    fn lock(&self) -> StdRwLockReadGuard<'_, State> {
        self.0.read().unwrap()
    }

    fn lock_mut(&self) -> StdRwLockWriteGuard<'_, State> {
        self.0.write().unwrap()
    }

    fn set_cached_validation(&self, slot_index: SlotIndex, v: proto::SCPValue, l: ValidationLevel) {
        let mut state = self.lock_mut();
        let x = state
            .validated_values
            .entry(slot_index)
            .or_insert_with(HashMap::new);
        x.insert(v, l);
    }

    fn get_cached_validation(
        &self,
        slot_index: SlotIndex,
        v: &proto::SCPValue,
    ) -> Option<ValidationLevel> {
        if let Some(x) = self.lock().validated_values.get(&slot_index) {
            x.get(v).cloned()
        } else {
            None
        }
    }

    fn purge_cached_validations(&self, slot_index: SlotIndex) {
        self.lock_mut().validated_values.remove(&slot_index);
    }
}

impl Consensus for SharedState {
    fn validate_ballot_value(
        &self,
        slot_index: SCPSlotIndex,
        value: &proto::SCPValue,
    ) -> ValidationLevel {
        self.get_cached_validation(slot_index, value).unwrap()
    }

    fn validate_nomination_value(
        &self,
        slot_index: SCPSlotIndex,
        value: &proto::SCPValue,
    ) -> ValidationLevel {
        self.get_cached_validation(slot_index, value).unwrap()
    }

    fn extract_valid_value(
        &self,
        slot_index: SCPSlotIndex,
        value: &proto::SCPValue,
    ) -> Option<proto::SCPValue> {
        let value = value.clone();
        let x = self.clone();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            x.lock()
                .handle
                .scp_handle_extract_valid_value(&slot_index, &value)
                .await
                .unwrap()
        })
    }

    fn combine_candidates(
        &self,
        slot_index: SCPSlotIndex,
        candidates: &HashSet<proto::SCPValue>,
    ) -> Option<proto::SCPValue> {
        let candidates = VecM::<SCPValue>(candidates.iter().cloned().collect());
        let x = self.clone();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            x.lock()
                .handle
                .scp_handle_combine_candidates(&slot_index, &candidates)
                .await
                .unwrap()
        })
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

fn to_validation_level(x: proto::SCPValidationLevel) -> ValidationLevel {
    match x {
        SCPValidationLevel::SCP_INVALID => ValidationLevel::Invalid,
        SCPValidationLevel::SCP_MAYBE_VALID => ValidationLevel::MaybeValid,
        SCPValidationLevel::SCP_FULLY_VALID => ValidationLevel::FullyValid,
    }
}
