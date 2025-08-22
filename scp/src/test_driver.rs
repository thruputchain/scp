use crate::*;
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::rc::Rc;
use std::time::Duration;

type TestDriverHandle = Rc<RefCell<TestDriver>>;

pub struct TestKernel {
    kernel: StellarKernel<TestDriverHandle, TestDriverHandle>,
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
        TestKernel {
            kernel: StellarKernel::new(node_id, is_validator, qset, driver.clone(), driver.clone()),
        }
    }
}

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
    pub timers: HashMap<i32, TimerData>,
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
}

impl Debug for TestDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestDriver").finish()
    }
}

impl Driver for Rc<RefCell<TestDriver>> {
    fn get_qset(&self, h: &proto::SCPHash) -> Option<&proto::SCPQuorumSet> {
        todo!("XXX")
    }

    fn to_short_string(&self, pk: &proto::SCPNodeID) -> String {
        todo!("XXX")
    }

    fn to_str_key(&self, pk: &proto::SCPNodeID, full_key: bool) -> String {
        todo!("XXX")
    }

    fn from_str_key(&self, s: &str) -> Result<proto::SCPNodeID, String> {
        todo!("XXX")
    } // XXX: get from KeyUtils.cpp

    fn get_value_string(&self, value: &proto::SCPValue) -> String {
        todo!("XXX")
    }

    fn get_hash_of(&self, opaque: &[u8]) -> proto::SCPHash {
        todo!("XXX")
    }

    fn get_node_weight(
        &self,
        node_id: &proto::SCPNodeID,
        qset: &proto::SCPQuorumSet,
        is_local_node: bool,
    ) -> u64 {
        todo!("XXX")
    }

    fn compute_hash_node(
        &self,
        slot_index: proto::SCPSlotIndex,
        prev: &proto::SCPValue,
        is_priority: bool,
        round_number: proto::SCPCount,
        node_id: &proto::SCPNodeID,
    ) -> u64 {
        todo!("XXX")
    }

    fn compute_value_hash(
        &self,
        slot_index: proto::SCPSlotIndex,
        prev: &proto::SCPValue,
        round_number: proto::SCPCount,
        value: &proto::SCPValue,
    ) -> u64 {
        todo!("XXX")
    }

    fn sign_envelope(&self, env: &mut proto::SCPEnvelope) {
        todo!("XXX")
    }

    fn emit_envelope(&self, env: proto::SCPEnvelope) {
        todo!("XXX")
    }

    fn compute_timeout(&self, round_number: proto::SCPCount, is_nomination: bool) -> Duration {
        todo!("XXX")
    }

    fn start_timer(
        &self,
        slot_index: SlotIndex,
        timer: Timer,
        timeout: Duration,
        cb: DriverCallback,
    ) {
        todo!("XXX")
    }

    fn stop_timer(&self, slot_index: SlotIndex, timer: Timer) {
        todo!("XXX")
    }
}

impl Consensus for Rc<RefCell<TestDriver>> {
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

impl ConsensusCb for Rc<RefCell<TestDriver>> {}

impl ConsensusNotify for Rc<RefCell<TestDriver>> {
    fn value_externalized(&self, slot_index: SlotIndex, value: &proto::SCPValue) {
        todo!()
    }

    fn started_ballot_proto(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        todo!()
    }

    fn accepted_ballot_prepared(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        todo!()
    }

    fn confirmed_ballot_prepared(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        todo!()
    }

    fn accepted_commit(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        todo!()
    }

    fn ballot_did_hear_from_quorum(&self, slot_index: SlotIndex, ballot: &proto::SCPBallot) {
        todo!()
    }

    fn nominating_value(&self, slot_index: SlotIndex, value: &proto::SCPValue) {
        todo!()
    }

    fn updated_candidate_value(&self, slot_index: SlotIndex, value: &proto::SCPValue) {
        todo!()
    }
}
