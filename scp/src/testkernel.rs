use crate::*;
use std::cell::RefCell;
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
        node_id: proto::SCPNodeID,
        is_validator: bool,
        qset: proto::SCPQuorumSet,
    ) -> TestKernel {
        let node_id_clone = node_id.clone();
        let driver = Rc::new(RefCell::new(TestDriver::new(
            Box::new(move |q: &proto::SCPNodeID| if q == &node_id_clone { 1000 } else { 1 }),
            Box::new(|_: &proto::SCPValue| 0),
        )));
        driver.borrow_mut().set_quorum_set(&qset);
        TestKernel {
            kernel: StellarKernel::new(node_id, is_validator, qset, driver.clone(), driver.clone()),
        }
    }

    pub fn proto(&mut self) -> StellarProto<'_, TestH, TestH> {
        self.kernel.proto()
    }

    /// used to test BallotProtocol and bypass nomination
    pub fn bump_state(&mut self, slot_index: SlotIndex, v: &proto::SCPValue) -> bool {
        self.kernel
            .proto()
            .slot_proto_or_create(slot_index)
            .bump_state(v, true)
    }

    pub fn nominate(
        &mut self,
        slot_index: SlotIndex,
        value: &proto::SCPValue,
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
    ) -> Option<proto::SCPValue> {
        self.kernel
            .proto()
            .slot_proto_or_create(slot_index)
            .get_latest_composite_candidate()
    }

    pub fn get_entire_current_state(&mut self, slot_index: SlotIndex) -> Vec<proto::SCPEnvelope> {
        if let Some(mut s) = self.kernel.proto().slot_proto(slot_index) {
            s.get_entire_current_state()
        } else {
            Vec::new()
        }
    }

    pub fn get_current_envelope(
        &mut self,
        slot_index: SlotIndex,
        node_id: &proto::SCPNodeID,
    ) -> proto::SCPEnvelope {
        let envelopes = self.get_entire_current_state(slot_index);
        envelopes
            .into_iter()
            .find(|env| &env.statement.node_id == node_id)
            .expect("not found")
    }

    pub fn get_nomination_leaders(&mut self, slot_index: SlotIndex) -> HashSet<proto::SCPNodeID> {
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
    pub priority_lookup: Box<dyn Fn(&proto::SCPNodeID) -> u64>,
    pub hash_value_calculator: Box<dyn Fn(&proto::SCPValue) -> u64>,

    // State storage
    pub quorum_sets: HashMap<proto::SCPHash, proto::SCPQuorumSet>,
    pub envelopes: Vec<proto::SCPEnvelope>,
    pub externalized_values: HashMap<SlotIndex, proto::SCPValue>,
    pub heard_from_quorums: HashMap<SlotIndex, Vec<proto::SCPBallot>>,

    // Timer management
    pub timers: HashMap<Timer, TimerData>,
    pub current_timer_offset: Duration,

    // Test-specific fields
    pub expected_candidates: HashSet<proto::SCPValue>,
    pub composite_value: Option<proto::SCPValue>,
}

impl TestDriver {
    pub fn new<P, Q>(priority_lookup: Box<P>, hash_value_calculator: Box<Q>) -> Self
    where
        P: Fn(&proto::SCPNodeID) -> u64 + 'static,
        Q: Fn(&proto::SCPValue) -> u64 + 'static,
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

    pub fn set_quorum_set(&mut self, qset: &proto::SCPQuorumSet) {
        self.quorum_sets
            .insert(proto::hash_xdr_scp(qset).unwrap(), qset.clone());
    }

    pub fn get_ballot_timer(&self) -> Option<TimerData> {
        self.timers.get(&Timer::Ballot).cloned()
    }

    pub fn bump_timer_offset(&mut self) -> Duration {
        // increase by more than the maximum timeout
        self.current_timer_offset += Duration::from_secs(60 * 60 * 5); // 5 hours
        self.current_timer_offset
    }

    pub fn has_ballot_timer(&self) -> bool {
        self.get_ballot_timer().is_some()
    }

    pub fn has_ballot_timer_upcoming(&self) -> bool {
        if let Some(x) = self.get_ballot_timer() {
            self.current_timer_offset < x.absolute_timeout
        } else {
            panic!("bug")
        }
    }
}

impl Debug for TestDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestDriver").finish()
    }
}

impl Driver for Rc<RefCell<TestDriver>> {
    fn get_qset(&self, qset_hash: &proto::SCPHash) -> Option<proto::SCPQuorumSet> {
        self.borrow().quorum_sets.get(qset_hash).cloned()
    }

    fn to_short_string(&self, _pk: &proto::SCPNodeID) -> String {
        panic!("not used")
    }

    fn to_str_key(&self, _pk: &proto::SCPNodeID, _full_key: bool) -> String {
        panic!("not used")
    }

    fn from_str_key(&self, _s: &str) -> Result<proto::SCPNodeID, String> {
        panic!("not used")
    } // XXX: get from KeyUtils.cpp

    fn get_value_string(&self, _value: &proto::SCPValue) -> String {
        panic!("not used")
    }

    fn get_hash_of(&self, opaque: &[u8]) -> proto::SCPHash {
        hash_bytes(opaque)
    }

    fn get_node_weight(
        &self,
        _node_id: &proto::SCPNodeID,
        _qset: &proto::SCPQuorumSet,
        _is_local_node: bool,
    ) -> u64 {
        panic!("not used")
    }

    fn compute_hash_node(
        &self,
        _slot_index: proto::SCPSlotIndex,
        _prev: &proto::SCPValue,
        is_priority: bool,
        _round_number: proto::SCPCount,
        node_id: &proto::SCPNodeID,
    ) -> u64 {
        if is_priority {
            (self.borrow().priority_lookup)(node_id)
        } else {
            0
        }
    }

    fn compute_value_hash(
        &self,
        _slot_index: proto::SCPSlotIndex,
        _prev: &proto::SCPValue,
        _round_number: proto::SCPCount,
        value: &proto::SCPValue,
    ) -> u64 {
        (self.borrow().hash_value_calculator)(value)
    }

    fn sign_envelope(&self, _: &mut proto::SCPEnvelope) {}

    fn emit_envelope(&self, env: proto::SCPEnvelope) {
        self.borrow_mut().envelopes.push(env);
    }

    fn compute_timeout(&self, round_number: proto::SCPCount, is_nomination: bool) -> Duration {
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

        let timeout_ms = initial_timeout_ms + (round_number - 1) * increment_ms;
        let timeout_ms = std::cmp::min(timeout_ms, TestKernel::MAX_TIMEOUT.as_millis() as u32);

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
    fn validate_ballot_value(&self, _: SCPSlotIndex, _: &proto::SCPValue) -> ValidationLevel {
        ValidationLevel::FullyValid
    }

    fn validate_nomination_value(&self, _: SCPSlotIndex, _: &proto::SCPValue) -> ValidationLevel {
        ValidationLevel::FullyValid
    }

    fn extract_valid_value(
        &self,
        _slot_index: SCPSlotIndex,
        _value: &proto::SCPValue,
    ) -> Option<proto::SCPValue> {
        todo!()
    }

    fn combine_candidates(
        &self,
        _slot_index: SCPSlotIndex,
        candidates: &HashSet<proto::SCPValue>,
    ) -> Option<proto::SCPValue> {
        let driver = self.borrow();

        assert_eq!(
            candidates.len(),
            driver.expected_candidates.len(),
            "candidates size mismatch"
        );

        for (candidate, expected) in candidates.iter().zip(driver.expected_candidates.iter()) {
            assert_eq!(candidate, expected, "candidate value mismatch");
        }

        assert!(
            driver.composite_value.is_some(),
            "composite value must not be empty"
        );

        driver.composite_value.clone()
    }
}

impl ConsensusCb for Rc<RefCell<TestDriver>> {}

impl ConsensusNotify for Rc<RefCell<TestDriver>> {
    fn value_externalized(&self, slot_index: SlotIndex, value: &proto::SCPValue) {
        let x = self
            .borrow_mut()
            .externalized_values
            .insert(slot_index, value.clone());
        assert!(x.is_none(), "value already externalized")
    }

    fn started_ballot_proto(&self, _slot_index: SlotIndex, _ballot: &proto::SCPBallot) {}

    fn accepted_ballot_prepared(&self, _slot_index: SlotIndex, _ballot: &proto::SCPBallot) {}

    fn confirmed_ballot_prepared(&self, _slot_index: SlotIndex, _ballot: &proto::SCPBallot) {}

    fn accepted_commit(&self, _slot_index: SlotIndex, _ballot: &proto::SCPBallot) {}

    fn ballot_did_hear_from_quorum(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        self.borrow_mut()
            .heard_from_quorums
            .entry(slot_index)
            .or_insert_with(Vec::new)
            .push(ballot.clone());
    }

    fn nominating_value(&self, _slot_index: SlotIndex, _value: &proto::SCPValue) {}

    fn updated_candidate_value(&self, _slot_index: SlotIndex, _value: &proto::SCPValue) {}
}
