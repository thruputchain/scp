use crate::*;
use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::rc::Rc;
use std::time::Duration;
use thruput_scp::*;

pub type TestH = Rc<RefCell<TestDriver>>;

pub struct TestKernel {
    kernel: StellarKernel<TestH, TestH>,
}

impl TestKernel {
    pub fn new(
        node_id: thruput_scp::SCPNodeID,
        qset: thruput_scp::SCPQuorumSet,
        is_validator: bool,
    ) -> TestKernel {
        let node_id_clone = node_id.clone();
        let driver = Rc::new(RefCell::new(TestDriver::new(
            Box::new(
                move |q: &thruput_scp::SCPNodeID| {
                    if q == &node_id_clone { 1000 } else { 1 }
                },
            ),
            Box::new(|_: &thruput_scp::SCPValue| 0),
        )));
        driver.borrow_mut().store_quorum_set(&qset);
        TestKernel {
            kernel: StellarKernel::new(
                node_id,
                is_validator,
                qset,
                driver.clone(),
                driver.clone(),
            ),
        }
    }

    pub fn driver(&mut self) -> RefMut<'_, TestDriver> {
        self.kernel.proto().cd.driver.borrow_mut()
    }

    pub fn proto(&mut self) -> StellarProto<'_, TestH, TestH> {
        self.kernel.proto()
    }

    /// used to test BallotProtocol and bypass nomination
    pub fn bump_state(
        &mut self,
        slot_index: SlotIndex,
        v: &thruput_scp::SCPValue,
    ) -> bool {
        self.kernel
            .proto()
            .slot_proto_or_create(slot_index)
            .bump_state(v, true)
    }

    pub fn nominate(
        &mut self,
        slot_index: SlotIndex,
        value: &thruput_scp::SCPValue,
        timed_out: bool,
    ) -> bool {
        self.kernel
            .proto()
            .slot_proto_or_create(slot_index)
            .nominate(value, value, timed_out)
    }

    pub fn get_latest_composite_candidate(
        &mut self,
        slot_index: SlotIndex,
    ) -> Option<thruput_scp::SCPValue> {
        self.kernel
            .proto()
            .slot_proto_or_create(slot_index)
            .get_latest_composite_candidate()
    }

    pub fn get_entire_current_state(
        &mut self,
        slot_index: SlotIndex,
    ) -> Vec<thruput_scp::SCPEnvelope> {
        if let Some(mut s) = self.kernel.proto().slot_proto(slot_index) {
            s.get_entire_current_state()
        } else {
            Vec::new()
        }
    }

    pub fn get_current_envelope(
        &mut self,
        slot_index: SlotIndex,
        node_id: &thruput_scp::SCPNodeID,
    ) -> thruput_scp::SCPEnvelope {
        let envelopes = self.get_entire_current_state(slot_index);
        envelopes
            .into_iter()
            .find(|env| &env.statement.node_id == node_id)
            .expect("not found")
    }

    pub fn get_nomination_leaders(
        &mut self,
        slot_index: SlotIndex,
    ) -> HashSet<thruput_scp::SCPNodeID> {
        if let Some(mut s) = self.kernel.proto().slot_proto(slot_index) {
            s.get_nomination_leaders()
        } else {
            HashSet::new()
        }
    }

    pub const MAX_TIMEOUT: Duration = Duration::from_millis((30 * 60) * 1000);
}

#[derive(Clone)]
pub struct TimerData {
    pub absolute_timeout: Duration,
    pub callback: DriverCallback,
}

pub struct TestDriver {
    // Timeout configuration
    pub initial_ballot_timeout_ms: u32,
    pub increment_ballot_timeout_ms: u32,
    pub initial_nomination_timeout_ms: u32,
    pub increment_nomination_timeout_ms: u32,

    // Callback functions
    pub priority_lookup: Box<dyn Fn(&thruput_scp::SCPNodeID) -> u64>,
    pub hash_value_calculator: Box<dyn Fn(&thruput_scp::SCPValue) -> u64>,

    // State storage
    pub quorum_sets: HashMap<thruput_scp::SCPHash, thruput_scp::SCPQuorumSet>,
    pub envelopes: Vec<thruput_scp::SCPEnvelope>,
    pub externalized_values: HashMap<SlotIndex, thruput_scp::SCPValue>,
    pub heard_from_quorums: HashMap<SlotIndex, Vec<thruput_scp::SCPBallot>>,

    // Timer management
    pub timers: HashMap<Timer, TimerData>,
    pub current_timer_offset: Duration,

    // Test-specific fields
    pub expected_candidates: HashSet<thruput_scp::SCPValue>,
    pub composite_value: Option<thruput_scp::SCPValue>,
    pub secret_key: Option<thruput_xdr::types::SecretKey>,
}

impl TestDriver {
    pub fn new<P, Q>(
        priority_lookup: Box<P>,
        hash_value_calculator: Box<Q>,
    ) -> Self
    where
        P: Fn(&thruput_scp::SCPNodeID) -> u64 + 'static,
        Q: Fn(&thruput_scp::SCPValue) -> u64 + 'static,
    {
        Self {
            initial_ballot_timeout_ms: 1000,
            increment_ballot_timeout_ms: 1000,
            initial_nomination_timeout_ms: 1000,
            increment_nomination_timeout_ms: 1000,
            priority_lookup,
            hash_value_calculator,
            quorum_sets: HashMap::new(),
            envelopes: Vec::new(),
            externalized_values: HashMap::new(),
            heard_from_quorums: HashMap::new(),
            timers: HashMap::new(),
            current_timer_offset: Duration::from_millis(0),
            expected_candidates: HashSet::new(),
            composite_value: None,
            secret_key: None,
        }
    }

    pub fn set_secret_key(
        &mut self,
        secret_key: thruput_xdr::types::SecretKey,
    ) {
        self.secret_key = Some(secret_key);
    }

    pub fn store_quorum_set(&mut self, qset: &thruput_scp::SCPQuorumSet) {
        self.quorum_sets
            .insert(scp_util::hash_xdr_scp(qset).unwrap(), qset.clone());
    }

    pub fn get_ballot_timer(&self) -> Option<TimerData> {
        self.timers.get(&Timer::Ballot).cloned()
    }

    pub fn get_nomination_timer(&self) -> Option<TimerData> {
        self.timers.get(&Timer::Nomination).cloned()
    }

    pub fn bump_timer_offset(&mut self) -> Duration {
        // increase by more than the maximum timeout
        self.current_timer_offset += Duration::from_secs(60 * 60 * 5); // 5 hours
        self.current_timer_offset
    }

    pub fn has_ballot_timer(&self) -> bool {
        self.get_ballot_timer().is_some()
    }

    pub fn has_nomination_timer(&self) -> bool {
        self.get_nomination_timer().is_some()
    }

    pub fn has_ballot_timer_upcoming(&self) -> bool {
        if let Some(x) = self.get_ballot_timer() {
            self.current_timer_offset < x.absolute_timeout
        } else {
            panic!("timer must be scheduled in the past or future")
        }
    }
}

impl Debug for TestDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestDriver").finish()
    }
}

impl Driver for Rc<RefCell<TestDriver>> {
    fn get_qset(
        &self,
        qset_hash: &thruput_scp::SCPHash,
    ) -> Option<thruput_scp::SCPQuorumSet> {
        self.borrow().quorum_sets.get(qset_hash).cloned()
    }

    fn to_short_string(&self, _pk: &thruput_scp::SCPNodeID) -> String {
        panic!("not used")
    }

    fn to_str_key(
        &self,
        pk: &thruput_scp::SCPNodeID,
        _full_key: bool,
    ) -> String {
        hex_abbrev(scp_util::public_key_bytes(pk))
    }

    fn from_str_key(&self, _s: &str) -> Result<thruput_scp::SCPNodeID, String> {
        panic!("not used")
    } // XXX: get from KeyUtils.cpp

    fn get_value_string(&self, value: &thruput_scp::SCPValue) -> String {
        hah(&value.0)
    }

    fn get_hash_of(&self, opaque: &[u8]) -> thruput_scp::SCPHash {
        hash_bytes(opaque)
    }

    fn get_node_weight(
        &self,
        node_id: &thruput_scp::SCPNodeID,
        qset: &thruput_scp::SCPQuorumSet,
        is_local_node: bool,
    ) -> u64 {
        get_node_weight(node_id, qset, is_local_node)
    }

    fn compute_hash_node(
        &self,
        _slot_index: thruput_scp::SCPSlotIndex,
        _prev: &thruput_scp::SCPValue,
        is_priority: bool,
        _round_number: thruput_scp::SCPCount,
        node_id: &thruput_scp::SCPNodeID,
    ) -> u64 {
        if is_priority {
            (self.borrow().priority_lookup)(node_id)
        } else {
            0
        }
    }

    fn compute_value_hash(
        &self,
        _slot_index: thruput_scp::SCPSlotIndex,
        _prev: &thruput_scp::SCPValue,
        _round_number: thruput_scp::SCPCount,
        value: &thruput_scp::SCPValue,
    ) -> u64 {
        (self.borrow().hash_value_calculator)(value)
    }

    fn sign_envelope(&self, env: &mut thruput_scp::SCPEnvelope) {
        if let Some(secret_key) = &self.borrow().secret_key {
            env.signature =
                scp_util::sign_xdr(secret_key, &env.statement).unwrap();
        }
    }

    fn emit_envelope(&self, env: thruput_scp::SCPEnvelope) {
        self.borrow_mut().envelopes.push(env);
    }

    fn compute_timeout(
        &self,
        round_number: thruput_scp::SCPCount,
        is_nomination: bool,
    ) -> Duration {
        let driver = self.borrow();
        let (initial_timeout_ms, increment_ms) = if is_nomination {
            (
                driver.initial_nomination_timeout_ms,
                driver.increment_nomination_timeout_ms,
            )
        } else {
            (
                driver.initial_ballot_timeout_ms,
                driver.increment_ballot_timeout_ms,
            )
        };

        let timeout_ms = initial_timeout_ms as u64
            + (round_number as u64 - 1) * increment_ms as u64;
        let timeout_ms = std::cmp::min(
            timeout_ms,
            TestKernel::MAX_TIMEOUT.as_millis() as u64,
        );

        Duration::from_millis(timeout_ms as u64)
    }

    fn start_timer(
        &self,
        _slot_index: SlotIndex,
        timer: Timer,
        timeout: Duration,
        cb: DriverCallback,
    ) {
        let mut driver = self.borrow_mut();
        let current_timer_offset = driver.current_timer_offset;
        driver.timers.insert(
            timer,
            TimerData {
                absolute_timeout: current_timer_offset + timeout,
                callback: cb,
            },
        );
    }

    fn stop_timer(&self, _slot_index: SlotIndex, timer: Timer) {
        self.borrow_mut().timers.remove(&timer);
    }
}

impl Consensus for Rc<RefCell<TestDriver>> {
    fn validate_ballot_value(
        &self,
        _: SCPSlotIndex,
        _: &thruput_scp::SCPValue,
    ) -> ValidationLevel {
        ValidationLevel::FullyValid
    }

    fn validate_nomination_value(
        &self,
        _: SCPSlotIndex,
        _: &thruput_scp::SCPValue,
    ) -> ValidationLevel {
        ValidationLevel::FullyValid
    }

    fn extract_valid_value(
        &self,
        _slot_index: SCPSlotIndex,
        _value: &thruput_scp::SCPValue,
    ) -> Option<thruput_scp::SCPValue> {
        todo!()
    }

    fn combine_candidates(
        &self,
        _slot_index: SCPSlotIndex,
        candidates: &HashSet<thruput_scp::SCPValue>,
    ) -> Option<thruput_scp::SCPValue> {
        let driver = self.borrow();

        assert_eq!(candidates, &driver.expected_candidates);

        assert!(
            driver.composite_value.is_some(),
            "composite value must not be empty"
        );

        driver.composite_value.clone()
    }
}

impl ConsensusNotify for Rc<RefCell<TestDriver>> {
    fn value_externalized(
        &self,
        slot_index: SlotIndex,
        value: &thruput_scp::SCPValue,
    ) {
        let x = self
            .borrow_mut()
            .externalized_values
            .insert(slot_index, value.clone());
        assert!(x.is_none(), "value already externalized")
    }

    fn started_ballot_proto(
        &self,
        _slot_index: SlotIndex,
        _ballot: &thruput_scp::SCPBallot,
    ) {
    }

    fn accepted_ballot_prepared(
        &self,
        _slot_index: SlotIndex,
        _ballot: &thruput_scp::SCPBallot,
    ) {
    }

    fn confirmed_ballot_prepared(
        &self,
        _slot_index: SlotIndex,
        _ballot: &thruput_scp::SCPBallot,
    ) {
    }

    fn accepted_commit(
        &self,
        _slot_index: SlotIndex,
        _ballot: &thruput_scp::SCPBallot,
    ) {
    }

    fn ballot_did_hear_from_quorum(
        &self,
        slot_index: SlotIndex,
        ballot: &thruput_scp::SCPBallot,
    ) {
        self.borrow_mut()
            .heard_from_quorums
            .entry(slot_index)
            .or_default()
            .push(ballot.clone());
    }

    fn nominating_value(
        &self,
        _slot_index: SlotIndex,
        _value: &thruput_scp::SCPValue,
    ) {
    }

    fn updated_candidate_value(
        &self,
        _slot_index: SlotIndex,
        _value: &thruput_scp::SCPValue,
    ) {
    }
}
