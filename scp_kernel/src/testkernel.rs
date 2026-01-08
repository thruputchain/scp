use crate::*;
use scp_proto::*;
use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::rc::Rc;
use std::time::Duration;

pub type TestH = Rc<RefCell<TestDriver>>;

pub struct TestKernel {
    kernel: StellarKernel<TestH, TestH>,
}

impl TestKernel {
    pub fn new(
        node_id: scp_proto::SCPNodeID,
        qset: scp_proto::SCPQuorumSet,
        is_validator: bool,
    ) -> TestKernel {
        let node_id_clone = node_id.clone();
        let driver = Rc::new(RefCell::new(TestDriver::new(
            Box::new(move |q: &scp_proto::SCPNodeID| if q == &node_id_clone { 1000 } else { 1 }),
            Box::new(|_: &scp_proto::SCPValue| 0),
        )));
        driver.borrow_mut().store_quorum_set(&qset);
        TestKernel {
            kernel: StellarKernel::new(node_id, is_validator, qset, driver.clone(), driver.clone()),
        }
    }

    pub fn driver(&mut self) -> RefMut<'_, TestDriver> {
        self.kernel.proto().cd.driver.borrow_mut()
    }

    pub fn proto(&mut self) -> StellarProto<'_, TestH, TestH> {
        self.kernel.proto()
    }

    /// used to test BallotProtocol and bypass nomination
    pub fn bump_state(&mut self, slot_index: SlotIndex, v: &scp_proto::SCPValue) -> bool {
        self.kernel
            .proto()
            .slot_proto_or_create(slot_index)
            .bump_state(v, true)
    }

    pub fn nominate(
        &mut self,
        slot_index: SlotIndex,
        value: &scp_proto::SCPValue,
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
    ) -> Option<scp_proto::SCPValue> {
        self.kernel
            .proto()
            .slot_proto_or_create(slot_index)
            .get_latest_composite_candidate()
    }

    pub fn get_entire_current_state(
        &mut self,
        slot_index: SlotIndex,
    ) -> Vec<scp_proto::SCPEnvelope> {
        if let Some(mut s) = self.kernel.proto().slot_proto(slot_index) {
            s.get_entire_current_state()
        } else {
            Vec::new()
        }
    }

    pub fn get_current_envelope(
        &mut self,
        slot_index: SlotIndex,
        node_id: &scp_proto::SCPNodeID,
    ) -> scp_proto::SCPEnvelope {
        let envelopes = self.get_entire_current_state(slot_index);
        envelopes
            .into_iter()
            .find(|env| &env.statement.node_id == node_id)
            .expect("not found")
    }

    pub fn get_nomination_leaders(
        &mut self,
        slot_index: SlotIndex,
    ) -> HashSet<scp_proto::SCPNodeID> {
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
    pub priority_lookup: Box<dyn Fn(&scp_proto::SCPNodeID) -> u64>,
    pub hash_value_calculator: Box<dyn Fn(&scp_proto::SCPValue) -> u64>,

    // State storage
    pub quorum_sets: HashMap<scp_proto::SCPHash, scp_proto::SCPQuorumSet>,
    pub envelopes: Vec<scp_proto::SCPEnvelope>,
    pub externalized_values: HashMap<SlotIndex, scp_proto::SCPValue>,
    pub heard_from_quorums: HashMap<SlotIndex, Vec<scp_proto::SCPBallot>>,

    // Timer management
    pub timers: HashMap<Timer, TimerData>,
    pub current_timer_offset: Duration,

    // Test-specific fields
    pub expected_candidates: HashSet<scp_proto::SCPValue>,
    pub composite_value: Option<scp_proto::SCPValue>,
}

impl TestDriver {
    pub fn new<P, Q>(priority_lookup: Box<P>, hash_value_calculator: Box<Q>) -> Self
    where
        P: Fn(&scp_proto::SCPNodeID) -> u64 + 'static,
        Q: Fn(&scp_proto::SCPValue) -> u64 + 'static,
    {
        Self {
            initial_ballot_timeout_ms: 1000,
            increment_ballot_timeout_ms: 1000,
            initial_nomination_timeout_ms: 1000,
            increment_nomination_timeout_ms: 1000,
            priority_lookup: priority_lookup,
            hash_value_calculator: hash_value_calculator,
            quorum_sets: HashMap::new(),
            envelopes: Vec::new(),
            externalized_values: HashMap::new(),
            heard_from_quorums: HashMap::new(),
            timers: HashMap::new(),
            current_timer_offset: Duration::from_millis(0),
            expected_candidates: HashSet::new(),
            composite_value: None,
        }
    }

    pub fn store_quorum_set(&mut self, qset: &scp_proto::SCPQuorumSet) {
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
    fn get_qset(&self, qset_hash: &scp_proto::SCPHash) -> Option<scp_proto::SCPQuorumSet> {
        self.borrow().quorum_sets.get(qset_hash).cloned()
    }

    fn to_short_string(&self, _pk: &scp_proto::SCPNodeID) -> String {
        panic!("not used")
    }

    fn to_str_key(&self, pk: &scp_proto::SCPNodeID, _full_key: bool) -> String {
        match pk {
            scp_proto::PublicKey::Ed25519 { ed25519 } => hex_abbrev(&ed25519.into_inner()[..]),
        }
    }

    fn from_str_key(&self, _s: &str) -> Result<scp_proto::SCPNodeID, String> {
        panic!("not used")
    } // XXX: get from KeyUtils.cpp

    fn get_value_string(&self, value: &scp_proto::SCPValue) -> String {
        format!("{}", hex_abbrev(&value.0))
    }

    fn get_hash_of(&self, opaque: &[u8]) -> scp_proto::SCPHash {
        hash_bytes(opaque)
    }

    fn get_node_weight(
        &self,
        node_id: &scp_proto::SCPNodeID,
        qset: &scp_proto::SCPQuorumSet,
        is_local_node: bool,
    ) -> u64 {
        get_node_weight(node_id, qset, is_local_node)
    }

    fn compute_hash_node(
        &self,
        _slot_index: scp_proto::SCPSlotIndex,
        _prev: &scp_proto::SCPValue,
        is_priority: bool,
        _round_number: scp_proto::SCPCount,
        node_id: &scp_proto::SCPNodeID,
    ) -> u64 {
        if is_priority {
            (self.borrow().priority_lookup)(node_id)
        } else {
            0
        }
    }

    fn compute_value_hash(
        &self,
        _slot_index: scp_proto::SCPSlotIndex,
        _prev: &scp_proto::SCPValue,
        _round_number: scp_proto::SCPCount,
        value: &scp_proto::SCPValue,
    ) -> u64 {
        (self.borrow().hash_value_calculator)(value)
    }

    fn sign_envelope(&self, _: &mut scp_proto::SCPEnvelope) {}

    fn emit_envelope(&self, env: scp_proto::SCPEnvelope) {
        self.borrow_mut().envelopes.push(env);
    }

    fn compute_timeout(&self, round_number: scp_proto::SCPCount, is_nomination: bool) -> Duration {
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

        let timeout_ms =
            initial_timeout_ms as u64 + (round_number as u64 - 1) * increment_ms as u64;
        let timeout_ms = std::cmp::min(timeout_ms, TestKernel::MAX_TIMEOUT.as_millis() as u64);

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
    fn validate_ballot_value(&self, _: SCPSlotIndex, _: &scp_proto::SCPValue) -> ValidationLevel {
        ValidationLevel::FullyValid
    }

    fn validate_nomination_value(
        &self,
        _: SCPSlotIndex,
        _: &scp_proto::SCPValue,
    ) -> ValidationLevel {
        ValidationLevel::FullyValid
    }

    fn extract_valid_value(
        &self,
        _slot_index: SCPSlotIndex,
        _value: &scp_proto::SCPValue,
    ) -> Option<scp_proto::SCPValue> {
        todo!()
    }

    fn combine_candidates(
        &self,
        _slot_index: SCPSlotIndex,
        candidates: &HashSet<scp_proto::SCPValue>,
    ) -> Option<scp_proto::SCPValue> {
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
    fn value_externalized(&self, slot_index: SlotIndex, value: &scp_proto::SCPValue) {
        let x = self
            .borrow_mut()
            .externalized_values
            .insert(slot_index, value.clone());
        assert!(x.is_none(), "value already externalized")
    }

    fn started_ballot_proto(&self, _slot_index: SlotIndex, _ballot: &scp_proto::SCPBallot) {}

    fn accepted_ballot_prepared(&self, _slot_index: SlotIndex, _ballot: &scp_proto::SCPBallot) {}

    fn confirmed_ballot_prepared(&self, _slot_index: SlotIndex, _ballot: &scp_proto::SCPBallot) {}

    fn accepted_commit(&self, _slot_index: SlotIndex, _ballot: &scp_proto::SCPBallot) {}

    fn ballot_did_hear_from_quorum(&self, slot_index: SlotIndex, ballot: &scp_proto::SCPBallot) {
        self.borrow_mut()
            .heard_from_quorums
            .entry(slot_index)
            .or_insert_with(Vec::new)
            .push(ballot.clone());
    }

    fn nominating_value(&self, _slot_index: SlotIndex, _value: &scp_proto::SCPValue) {}

    fn updated_candidate_value(&self, _slot_index: SlotIndex, _value: &scp_proto::SCPValue) {}
}
