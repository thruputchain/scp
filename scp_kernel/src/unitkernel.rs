use crate::*;
use scp_proto::*;
use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::rc::Rc;
use std::time::Duration;
use xdr_common::*;

pub type UnitTestH = Rc<RefCell<UnitTestDriver>>;

pub struct UnitTestKernel {
    kernel: StellarKernel<UnitTestH, UnitTestH>,
}

impl UnitTestKernel {
    pub fn new(
        node_id: scp_proto::SCPNodeID,
        qset: scp_proto::SCPQuorumSet,
        is_validator: bool,
    ) -> UnitTestKernel {
        let driver = Rc::new(RefCell::new(UnitTestDriver::new()));
        driver.borrow_mut().store_quorum_set(&qset);
        UnitTestKernel {
            kernel: StellarKernel::new(
                node_id,
                is_validator,
                qset,
                driver.clone(),
                driver.clone(),
            ),
        }
    }

    pub fn driver(&mut self) -> RefMut<'_, UnitTestDriver> {
        self.kernel.proto().cd.driver.borrow_mut()
    }

    pub fn proto(&mut self) -> StellarProto<'_, UnitTestH, UnitTestH> {
        self.kernel.proto()
    }

    pub fn get_latest_composite_candidate(
        &mut self,
        _slot_index: SlotIndex,
    ) -> Option<scp_proto::SCPValue> {
        Some(xdr_common::BytesM(vec![]))
    }
}

pub struct UnitTestDriver {
    // Timeout configuration
    pub initial_ballot_timeout_ms: u32,
    pub increment_ballot_timeout_ms: u32,
    pub initial_nomination_timeout_ms: u32,
    pub increment_nomination_timeout_ms: u32,
    // State storage
    pub quorum_sets: HashMap<scp_proto::SCPHash, scp_proto::SCPQuorumSet>,
}

impl Default for UnitTestDriver {
    fn default() -> Self {
        Self {
            initial_ballot_timeout_ms: 1000,
            increment_ballot_timeout_ms: 1000,
            initial_nomination_timeout_ms: 1000,
            increment_nomination_timeout_ms: 1000,
            quorum_sets: HashMap::new(),
        }
    }
}

impl UnitTestDriver {
    const MAX_TIMEOUT: Duration = Duration::from_millis((30 * 60) * 1000);

    pub fn new() -> Self {
        Self::default()
    }

    pub fn store_quorum_set(&mut self, qset: &scp_proto::SCPQuorumSet) {
        self.quorum_sets
            .insert(scp_util::hash_xdr_scp(qset).unwrap(), qset.clone());
    }
}

impl Debug for UnitTestDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnitTestDriver").finish()
    }
}

impl Driver for Rc<RefCell<UnitTestDriver>> {
    fn get_qset(
        &self,
        qset_hash: &scp_proto::SCPHash,
    ) -> Option<scp_proto::SCPQuorumSet> {
        self.borrow().quorum_sets.get(qset_hash).cloned()
    }

    fn to_short_string(&self, _pk: &scp_proto::SCPNodeID) -> String {
        panic!("not used")
    }

    fn to_str_key(&self, pk: &scp_proto::SCPNodeID, _full_key: bool) -> String {
        match pk {
            SCPPublicKey::Ed25519 { ed25519 } => {
                hex_abbrev(&ed25519.into_inner()[..])
            }
            _ => panic!("unreachable"),
        }
    }

    fn from_str_key(&self, _s: &str) -> Result<scp_proto::SCPNodeID, String> {
        panic!("not used")
    } // XXX: get from KeyUtils.cpp

    fn get_value_string(&self, value: &scp_proto::SCPValue) -> String {
        hah(&value.0)
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
        slot_index: scp_proto::SCPSlotIndex,
        prev: &scp_proto::SCPValue,
        is_priority: bool,
        round_number: scp_proto::SCPCount,
        node_id: &scp_proto::SCPNodeID,
    ) -> u64 {
        // XXX: is mPriorityLookupForTesting relevant here?
        hash_helper(
            slot_index,
            prev,
            |v| {
                v.push(BytesM(
                    scp_util::marshal_xdr(if is_priority {
                        &HASH_P
                    } else {
                        &HASH_N
                    })
                    .unwrap()
                    .to_vec(),
                ));
                v.push(BytesM(
                    scp_util::marshal_xdr(&round_number).unwrap().to_vec(),
                ));
                v.push(BytesM(
                    scp_util::marshal_xdr(node_id).unwrap().to_vec(),
                ));
            },
            |b| self.get_hash_of(b),
        )
    }

    fn compute_value_hash(
        &self,
        slot_index: scp_proto::SCPSlotIndex,
        prev: &scp_proto::SCPValue,
        round_number: scp_proto::SCPCount,
        value: &scp_proto::SCPValue,
    ) -> u64 {
        hash_helper(
            slot_index,
            prev,
            |v| {
                v.push(BytesM(
                    scp_util::marshal_xdr(&HASH_K).unwrap().to_vec(),
                ));
                v.push(BytesM(
                    scp_util::marshal_xdr(&round_number).unwrap().to_vec(),
                ));
                v.push(BytesM(scp_util::marshal_xdr(value).unwrap().to_vec()));
            },
            |b| self.get_hash_of(b),
        )
    }

    fn sign_envelope(&self, _: &mut scp_proto::SCPEnvelope) {}

    fn emit_envelope(&self, _: scp_proto::SCPEnvelope) {}

    fn compute_timeout(
        &self,
        round_number: scp_proto::SCPCount,
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
            UnitTestDriver::MAX_TIMEOUT.as_millis() as u64,
        );

        Duration::from_millis(timeout_ms as u64)
    }

    fn start_timer(
        &self,
        _slot_index: SlotIndex,
        _timer: Timer,
        _timeout: Duration,
        _cb: DriverCallback,
    ) {
    }

    fn stop_timer(&self, _slot_index: SlotIndex, _timer: Timer) {}
}

impl Consensus for Rc<RefCell<UnitTestDriver>> {
    fn validate_ballot_value(
        &self,
        _: SCPSlotIndex,
        _: &scp_proto::SCPValue,
    ) -> ValidationLevel {
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
        None
    }

    fn combine_candidates(
        &self,
        _slot_index: SCPSlotIndex,
        _candidates: &HashSet<scp_proto::SCPValue>,
    ) -> Option<scp_proto::SCPValue> {
        None
    }
}

impl ConsensusNotify for Rc<RefCell<UnitTestDriver>> {
    fn value_externalized(
        &self,
        _slot_index: SlotIndex,
        _value: &scp_proto::SCPValue,
    ) {
    }

    fn started_ballot_proto(
        &self,
        _slot_index: SlotIndex,
        _ballot: &scp_proto::SCPBallot,
    ) {
    }

    fn accepted_ballot_prepared(
        &self,
        _slot_index: SlotIndex,
        _ballot: &scp_proto::SCPBallot,
    ) {
    }

    fn confirmed_ballot_prepared(
        &self,
        _slot_index: SlotIndex,
        _ballot: &scp_proto::SCPBallot,
    ) {
    }

    fn accepted_commit(
        &self,
        _slot_index: SlotIndex,
        _ballot: &scp_proto::SCPBallot,
    ) {
    }

    fn ballot_did_hear_from_quorum(
        &self,
        _slot_index: SlotIndex,
        _ballot: &scp_proto::SCPBallot,
    ) {
    }

    fn nominating_value(
        &self,
        _slot_index: SlotIndex,
        _value: &scp_proto::SCPValue,
    ) {
    }

    fn updated_candidate_value(
        &self,
        _slot_index: SlotIndex,
        _value: &scp_proto::SCPValue,
    ) {
    }
}
