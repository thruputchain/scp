use scp_kernel::*;
use std::cell::{RefCell, RefMut};
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct Server {}

impl Server {
    pub fn new() -> Server {
        Server {}
    }
}

#[derive(Debug, Clone)]
pub struct AServer(pub Arc<RwLock<Server>>);

impl Consensus for AServer {
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

impl ConsensusNotify for AServer {
    fn value_externalized(&self, _slot_index: SlotIndex, _value: &proto::SCPValue) {}
    fn started_ballot_proto(&self, _slot_index: SlotIndex, _ballot: &proto::SCPBallot) {}
    fn accepted_ballot_prepared(&self, _slot_index: SlotIndex, _ballot: &proto::SCPBallot) {}
    fn confirmed_ballot_prepared(&self, _slot_index: SlotIndex, _ballot: &proto::SCPBallot) {}
    fn accepted_commit(&self, _slot_index: SlotIndex, _ballot: &proto::SCPBallot) {}
    fn ballot_did_hear_from_quorum(&self, _slot_index: SlotIndex, _ballot: &proto::SCPBallot) {}
    fn nominating_value(&self, _slot_index: SlotIndex, _value: &proto::SCPValue) {}
    fn updated_candidate_value(&self, _slot_index: SlotIndex, _value: &proto::SCPValue) {}
}
