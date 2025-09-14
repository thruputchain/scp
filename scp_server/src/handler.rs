use crate::controller::*;
use futures::future::join_all;
use scp_kernel::*;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::sync::RwLockReadGuard as StdRwLockReadGuard;
use std::sync::RwLockWriteGuard as StdRwLockWriteGuard;
use std::time::Duration;
use tokio::sync::RwLock as TokioRwLock;
use tokio::sync::RwLockReadGuard as TokioRwLockReadGuard;
use tokio::sync::RwLockWriteGuard as TokioRwLockWriteGuard;
use tokio::sync::mpsc::{Receiver, Sender, UnboundedSender};
use xdr_common::*;

// ValidateClient

#[derive(Clone)]
pub struct ValidateClient {
    rx_validate_envelope: Arc<TokioRwLock<Receiver<ValidateEnvelope>>>,
    tx_envelope_validated: Arc<TokioRwLock<Sender<EnvelopeValidated>>>,
    client: SCP_Handle_SCP_Handle1_Client,
}

impl ValidateClient {
    pub fn new(
        rx_validate_envelope: Receiver<ValidateEnvelope>,
        tx_envelope_validated: Sender<EnvelopeValidated>,
        client: SCP_Handle_SCP_Handle1_Client,
    ) -> ValidateClient {
        ValidateClient {
            rx_validate_envelope: Arc::new(TokioRwLock::new(rx_validate_envelope)),
            tx_envelope_validated: Arc::new(TokioRwLock::new(tx_envelope_validated)),
            client,
        }
    }

    pub async fn start(&self) {
        let twin = self.clone();
        tokio::spawn(async move {
            let mut rx_validate_envelope = twin.rx_validate_envelope.write().await;
            loop {
                let env = rx_validate_envelope.recv().await.unwrap();
                tokio::spawn({
                    let x = twin.clone();
                    async move { x.validate_envelope(env).await }
                });
            }
        });
    }

    async fn validate_envelope(&self, req: ValidateEnvelope) {
        let slot_index = req.env.statement.slot_index;

        // call rpc validates in parallel, join
        let mut handles = Vec::new();
        for v in req.validate_nomination_values {
            handles.push(tokio::spawn({
                let x = self.clone();
                async move {
                    (
                        slot_index,
                        v.clone(),
                        x.client
                            .scp_handle_validate_nomination_value(&slot_index, &v)
                            .await
                            .unwrap(),
                    )
                }
            }));
        }
        for v in req.validate_ballot_values {
            handles.push(tokio::spawn({
                let x = self.clone();
                async move {
                    (
                        slot_index,
                        v.clone(),
                        x.client
                            .scp_handle_validate_ballot_value(&slot_index, &v)
                            .await
                            .unwrap(),
                    )
                }
            }));
        }
        let validated_values: Vec<_> = join_all(handles)
            .await
            .into_iter()
            .map(|r| {
                let (slot_index, value, svl) = r.unwrap();
                (slot_index, value, to_validation_level(svl))
            })
            .collect();

        // send reply on backchannel
        self.tx_envelope_validated
            .write()
            .await
            .send(EnvelopeValidated {
                env: req.env,
                validated_values,
            });
    }
}

// HandleClient

#[derive(Clone)]
pub struct HandleClient(Arc<TokioRwLock<HandleClientState>>);

struct HandleClientState {
    rx_request: Receiver<HandleRequest>,
    tx_reply: Sender<HandleReply>,
    scp_handle: SCP_Handle_SCP_Handle1_Client,
}

impl HandleClientState {
    async fn handle_request(&self, req: HandleRequest) {
        match req {
            HandleRequest::EmitEnvelope(env) => {
                self.scp_handle
                    .scp_handle_emit_envelope(&env)
                    .await
                    .unwrap();
                self.tx_reply
                    .send(HandleReply::EmitEnvelope())
                    .await
                    .unwrap();
            }
            HandleRequest::ValidateBallotValue(slot_index, value) => {
                let r = self
                    .scp_handle
                    .scp_handle_validate_ballot_value(&slot_index, &value)
                    .await
                    .unwrap();
                self.tx_reply
                    .send(HandleReply::ValidateBallotValue(r))
                    .await
                    .unwrap();
            }
            HandleRequest::ValidateNominationValue(slot_index, value) => {
                let r = self
                    .scp_handle
                    .scp_handle_validate_nomination_value(&slot_index, &value)
                    .await
                    .unwrap();
                self.tx_reply
                    .send(HandleReply::ValidateNominationValue(r))
                    .await
                    .unwrap();
            }
            HandleRequest::ExtractValidValue(slot_index, value) => {
                let r = self
                    .scp_handle
                    .scp_handle_extract_valid_value(&slot_index, &value)
                    .await
                    .unwrap();
                self.tx_reply
                    .send(HandleReply::ExtractValidValue(r))
                    .await
                    .unwrap();
            }
            HandleRequest::CombineCandidates(slot_index, values) => {
                let r = self
                    .scp_handle
                    .scp_handle_combine_candidates(&slot_index, &values)
                    .await
                    .unwrap();
                self.tx_reply
                    .send(HandleReply::CombineCandidates(r))
                    .await
                    .unwrap();
            }
            HandleRequest::GetQuorumSet(hash) => {
                let r = self
                    .scp_handle
                    .scp_handle_get_quorum_set(&hash)
                    .await
                    .unwrap();
                self.tx_reply
                    .send(HandleReply::GetQuorumSet(r))
                    .await
                    .unwrap();
            }
            HandleRequest::ValueExternalized(slot_index, value) => {
                self.scp_handle
                    .scp_handle_value_externalized(&slot_index, &value)
                    .await
                    .unwrap();
                self.tx_reply
                    .send(HandleReply::ValueExternalized())
                    .await
                    .unwrap();
            }
            HandleRequest::StartedBallotProto(slot_index, ballot) => {
                self.scp_handle
                    .scp_handle_started_ballot_proto(&slot_index, &ballot)
                    .await
                    .unwrap();
                self.tx_reply
                    .send(HandleReply::StartedBallotProto())
                    .await
                    .unwrap();
            }
            HandleRequest::AcceptedBallotPrepared(slot_index, ballot) => {
                self.scp_handle
                    .scp_handle_accepted_ballot_prepared(&slot_index, &ballot)
                    .await
                    .unwrap();
                self.tx_reply
                    .send(HandleReply::AcceptedBallotPrepared())
                    .await
                    .unwrap();
            }
            HandleRequest::ConfirmedBallotPrepared(slot_index, ballot) => {
                self.scp_handle
                    .scp_handle_confirmed_ballot_prepared(&slot_index, &ballot)
                    .await
                    .unwrap();
                self.tx_reply
                    .send(HandleReply::ConfirmedBallotPrepared())
                    .await
                    .unwrap();
            }
            HandleRequest::AcceptedCommit(slot_index, ballot) => {
                self.scp_handle
                    .scp_handle_accepted_commit(&slot_index, &ballot)
                    .await
                    .unwrap();
                self.tx_reply
                    .send(HandleReply::AcceptedCommit())
                    .await
                    .unwrap();
            }
            HandleRequest::BallotDidHearFromQuorum(slot_index, ballot) => {
                self.scp_handle
                    .scp_handle_ballot_did_hear_from_quorum(&slot_index, &ballot)
                    .await
                    .unwrap();
                self.tx_reply
                    .send(HandleReply::BallotDidHearFromQuorum())
                    .await
                    .unwrap();
            }
            HandleRequest::NominatingValue(slot_index, value) => {
                self.scp_handle
                    .scp_handle_nominating_value(&slot_index, &value)
                    .await
                    .unwrap();
                self.tx_reply
                    .send(HandleReply::NominatingValue())
                    .await
                    .unwrap();
            }
            HandleRequest::UpdatedCandidateValue(slot_index, value) => {
                self.scp_handle
                    .scp_handle_updated_candidate_value(&slot_index, &value)
                    .await
                    .unwrap();
                self.tx_reply
                    .send(HandleReply::UpdatedCandidateValue())
                    .await
                    .unwrap();
            }
        }
    }
}

impl HandleClient {
    pub fn new(
        rx_request: Receiver<HandleRequest>,
        tx_reply: Sender<HandleReply>,
        scp_handle: SCP_Handle_SCP_Handle1_Client,
    ) -> HandleClient {
        HandleClient(Arc::new(TokioRwLock::new(HandleClientState {
            rx_request,
            tx_reply,
            scp_handle,
        })))
    }

    pub async fn start(&self) {
        let self_proc = self.clone();
        tokio::spawn(async move {
            let mut lock = self_proc.lock_mut().await;
            loop {
                let req = lock.rx_request.recv().await.unwrap();
                lock.handle_request(req);
            }
        });
    }

    async fn lock(&self) -> TokioRwLockReadGuard<'_, HandleClientState> {
        self.0.read().await
    }

    async fn lock_mut(&self) -> TokioRwLockWriteGuard<'_, HandleClientState> {
        self.0.write().await
    }
}

pub enum HandleRequest {
    EmitEnvelope(SCPEnvelope),
    ValidateBallotValue(SCPSlotIndex, SCPValue),
    ValidateNominationValue(SCPSlotIndex, SCPValue),
    ExtractValidValue(SCPSlotIndex, SCPValue),
    CombineCandidates(SCPSlotIndex, SCPValues),
    GetQuorumSet(SCPHash),
    //
    ValueExternalized(SlotIndex, SCPValue),
    StartedBallotProto(SlotIndex, SCPBallot),
    AcceptedBallotPrepared(SlotIndex, SCPBallot),
    ConfirmedBallotPrepared(SlotIndex, SCPBallot),
    AcceptedCommit(SlotIndex, SCPBallot),
    BallotDidHearFromQuorum(SlotIndex, SCPBallot),
    NominatingValue(SlotIndex, SCPValue),
    UpdatedCandidateValue(SlotIndex, SCPValue),
}

pub enum HandleReply {
    EmitEnvelope(),
    ValidateBallotValue(SCPValidationLevel),
    ValidateNominationValue(SCPValidationLevel),
    ExtractValidValue(SCPOptValue),
    CombineCandidates(SCPOptValue),
    GetQuorumSet(SCPOptQuorumSet),
    //
    ValueExternalized(),
    StartedBallotProto(),
    AcceptedBallotPrepared(),
    ConfirmedBallotPrepared(),
    AcceptedCommit(),
    BallotDidHearFromQuorum(),
    NominatingValue(),
    UpdatedCandidateValue(),
}

// HandleKernel

#[derive(Debug, Clone)]
pub struct HandleKernel(Arc<StdRwLock<HandleKernelState>>);

struct HandleKernelState {
    pub tx_handle_request: Sender<HandleRequest>,
    pub rx_handle_reply: Receiver<HandleReply>,
    //
    max_timeout: Duration,
    initial_ballot_timeout: Duration,
    increment_ballot_timeout: Duration,
    initial_nomination_timeout: Duration,
    increment_nomination_timeout: Duration,
    //
    secret_key: proto::SecretKey,
    //
    validated_values: ValidatedValues,
    //
    tx_switch_timer: UnboundedSender<SwitchTimer>,
}

impl fmt::Debug for HandleKernelState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HandleKernelState")
            .field("tx_handle_request", &self.tx_handle_request)
            .field("rx_handle_reply", &self.rx_handle_reply)
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
            .field("validated_values", &self.validated_values)
            // skipping driver_calling_back
            .finish()
    }
}

pub enum SwitchTimer {
    StartTimer(SlotIndex, Timer, Duration, DriverCallback),
    StopTimer(SlotIndex, Timer),
}

impl HandleKernel {
    pub fn new(
        tx_handle_request: Sender<HandleRequest>,
        rx_handle_reply: Receiver<HandleReply>,
        tx_switch_timer: UnboundedSender<SwitchTimer>,
        cfg: &proto::SCPConfig,
        secret_key: &proto::SecretKey,
    ) -> HandleKernel {
        HandleKernel(Arc::new(StdRwLock::new(HandleKernelState {
            tx_handle_request,
            rx_handle_reply,
            max_timeout: Duration::from_millis(cfg.max_timeout_ms),
            initial_ballot_timeout: Duration::from_millis(cfg.initial_ballot_timeout_ms),
            increment_ballot_timeout: Duration::from_millis(cfg.increment_ballot_timeout_ms),
            initial_nomination_timeout: Duration::from_millis(cfg.initial_nomination_timeout_ms),
            increment_nomination_timeout: Duration::from_millis(
                cfg.increment_nomination_timeout_ms,
            ),
            secret_key: secret_key.clone(),
            validated_values: HashMap::new(),
            tx_switch_timer,
        })))
    }

    fn lock(&self) -> StdRwLockReadGuard<'_, HandleKernelState> {
        self.0.read().unwrap()
    }

    fn lock_mut(&self) -> StdRwLockWriteGuard<'_, HandleKernelState> {
        self.0.write().unwrap()
    }

    pub fn set_cached_validation(
        &self,
        slot_index: SlotIndex,
        v: proto::SCPValue,
        l: ValidationLevel,
    ) {
        let mut state = self.lock_mut();
        let x = state
            .validated_values
            .entry(slot_index)
            .or_insert_with(HashMap::new);
        x.insert(v, l);
    }

    pub fn get_cached_validation(
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

    pub fn purge_cached_validations(&self, slot_index: SlotIndex) {
        self.lock_mut().validated_values.remove(&slot_index);
    }
}

impl Consensus for HandleKernel {
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
        self.lock()
            .tx_handle_request
            .blocking_send(HandleRequest::ExtractValidValue(slot_index, value.clone()))
            .unwrap();
        if let HandleReply::ExtractValidValue(r) =
            self.lock_mut().rx_handle_reply.blocking_recv().unwrap()
        {
            r
        } else {
            panic!("bug")
        }
    }

    fn combine_candidates(
        &self,
        slot_index: SCPSlotIndex,
        candidates: &HashSet<proto::SCPValue>,
    ) -> Option<proto::SCPValue> {
        let candidates = VecM::<SCPValue>(candidates.iter().cloned().collect());
        self.lock()
            .tx_handle_request
            .blocking_send(HandleRequest::CombineCandidates(slot_index, candidates))
            .unwrap();
        if let HandleReply::CombineCandidates(r) =
            self.lock_mut().rx_handle_reply.blocking_recv().unwrap()
        {
            r
        } else {
            panic!("bug")
        }
    }
}

impl ConsensusNotify for HandleKernel {
    fn value_externalized(&self, slot_index: SlotIndex, value: &proto::SCPValue) {
        self.lock()
            .tx_handle_request
            .blocking_send(HandleRequest::ValueExternalized(slot_index, value.clone()))
            .unwrap();
        if let HandleReply::ValueExternalized() =
            self.lock_mut().rx_handle_reply.blocking_recv().unwrap()
        {
        } else {
            panic!("bug")
        }
    }

    fn started_ballot_proto(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        self.lock()
            .tx_handle_request
            .blocking_send(HandleRequest::StartedBallotProto(
                slot_index,
                ballot.clone(),
            ))
            .unwrap();
        if let HandleReply::StartedBallotProto() =
            self.lock_mut().rx_handle_reply.blocking_recv().unwrap()
        {
        } else {
            panic!("bug")
        }
    }

    fn accepted_ballot_prepared(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        self.lock()
            .tx_handle_request
            .blocking_send(HandleRequest::AcceptedBallotPrepared(
                slot_index,
                ballot.clone(),
            ))
            .unwrap();
        if let HandleReply::AcceptedBallotPrepared() =
            self.lock_mut().rx_handle_reply.blocking_recv().unwrap()
        {
        } else {
            panic!("bug")
        }
    }

    fn confirmed_ballot_prepared(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        self.lock()
            .tx_handle_request
            .blocking_send(HandleRequest::ConfirmedBallotPrepared(
                slot_index,
                ballot.clone(),
            ))
            .unwrap();
        if let HandleReply::ConfirmedBallotPrepared() =
            self.lock_mut().rx_handle_reply.blocking_recv().unwrap()
        {
        } else {
            panic!("bug")
        }
    }

    fn accepted_commit(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        self.lock()
            .tx_handle_request
            .blocking_send(HandleRequest::AcceptedCommit(slot_index, ballot.clone()))
            .unwrap();
        if let HandleReply::AcceptedCommit() =
            self.lock_mut().rx_handle_reply.blocking_recv().unwrap()
        {
        } else {
            panic!("bug")
        }
    }

    fn ballot_did_hear_from_quorum(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        self.lock()
            .tx_handle_request
            .blocking_send(HandleRequest::BallotDidHearFromQuorum(
                slot_index,
                ballot.clone(),
            ))
            .unwrap();
        if let HandleReply::BallotDidHearFromQuorum() =
            self.lock_mut().rx_handle_reply.blocking_recv().unwrap()
        {
        } else {
            panic!("bug")
        }
    }

    fn nominating_value(&self, slot_index: SlotIndex, value: &proto::SCPValue) {
        self.lock()
            .tx_handle_request
            .blocking_send(HandleRequest::NominatingValue(slot_index, value.clone()))
            .unwrap();
        if let HandleReply::NominatingValue() =
            self.lock_mut().rx_handle_reply.blocking_recv().unwrap()
        {
        } else {
            panic!("bug")
        }
    }

    fn updated_candidate_value(&self, slot_index: SlotIndex, value: &proto::SCPValue) {
        self.lock()
            .tx_handle_request
            .blocking_send(HandleRequest::UpdatedCandidateValue(
                slot_index,
                value.clone(),
            ))
            .unwrap();
        if let HandleReply::UpdatedCandidateValue() =
            self.lock_mut().rx_handle_reply.blocking_recv().unwrap()
        {
        } else {
            panic!("bug")
        }
    }
}

impl Driver for HandleKernel {
    fn get_qset(&self, h: &proto::SCPHash) -> Option<proto::SCPQuorumSet> {
        self.lock()
            .tx_handle_request
            .blocking_send(HandleRequest::GetQuorumSet(h.clone()))
            .unwrap();
        if let HandleReply::GetQuorumSet(q) =
            self.lock_mut().rx_handle_reply.blocking_recv().unwrap()
        {
            q
        } else {
            panic!("bug")
        }
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
        self.lock()
            .tx_handle_request
            .blocking_send(HandleRequest::EmitEnvelope(env.clone()))
            .unwrap();
        if let HandleReply::EmitEnvelope() =
            self.lock_mut().rx_handle_reply.blocking_recv().unwrap()
        {
        } else {
            panic!("bug")
        }
    }

    fn start_timer(
        &self,
        slot_index: SlotIndex,
        timer: Timer,
        timeout: Duration,
        cb: DriverCallback,
    ) {
        self.lock()
            .tx_switch_timer
            .send(SwitchTimer::StartTimer(slot_index, timer, timeout, cb));
    }

    fn stop_timer(&self, slot_index: SlotIndex, timer: Timer) {
        self.lock()
            .tx_switch_timer
            .send(SwitchTimer::StopTimer(slot_index, timer));
    }
}

fn to_validation_level(x: proto::SCPValidationLevel) -> ValidationLevel {
    match x {
        SCPValidationLevel::SCP_INVALID => ValidationLevel::Invalid,
        SCPValidationLevel::SCP_MAYBE_VALID => ValidationLevel::MaybeValid,
        SCPValidationLevel::SCP_FULLY_VALID => ValidationLevel::FullyValid,
    }
}
