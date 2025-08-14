use crate::*;
use std::collections::HashMap;

pub type SlotIndex = u64;

// The number of times the timer has to expire before we consider the node
// missing. Used for reporting purposes only.
pub const NUM_TIMEOUTS_THRESHOLD_FOR_REPORTING: i64 = 2;

pub struct Slot {
    pub slot_index: SlotIndex,
    pub ballot: Ballot,
}

impl Slot {
    pub fn new() -> Self {
        Slot {
            slot_index: 0,
            ballot: Ballot::new(),
        }
    }
}

pub struct SlotProto<'a, C: Consensus, D: Driver> {
    pub cd: &'a ConsensusDriver<'a, C, D>,
    pub local: &'a mut Local,
    pub slot: &'a mut Slot,
}

impl<'a, C: Consensus, D: Driver> SlotProto<'a, C, D> {
    pub fn ballot_proto(&mut self) -> BallotProto<'_, 'a, C, D> {
        BallotProto(self)
    }

    pub fn index(&self) -> SlotIndex {
        self.slot.slot_index
    }

    pub fn ballot(&self) -> &Ballot {
        &self.slot.ballot
    }

    pub fn ballot_mut(&mut self) -> &mut Ballot {
        &mut self.slot.ballot
    }

    pub fn driver(&self) -> &D {
        self.cd.driver
    }

    pub fn consensus(&self) -> &C {
        self.cd.consensus
    }

    pub fn timer_expired(&mut self, timer: Timer) {
        match timer {
            Timer::Ballot => self.ballot_proto().timer_expired(),
            Timer::Nomination => panic!("XXX"),
        }
    }

    pub fn get_qset_from_statement(
        &self,
        statement: &proto::SCPStatement,
    ) -> Option<proto::SCPQuorumSet> {
        panic!("XXX")
    }

    pub fn record_envelope(&mut self, env: &proto::SCPEnvelope) {
        panic!("XXX")
    }

    pub fn set_fully_validated(&mut self, validated: bool) {
        panic!("XXX")
    }

    pub fn get_latest_composite_candidate(&self) -> Option<&proto::SCPValue> {
        panic!("XXX")
    }

    pub fn create_envelope(&self, statement: proto::SCPStatement) -> proto::SCPEnvelope {
        panic!("XXX")
    }

    pub fn is_fully_validated(&self) -> bool {
        panic!("XXX")
    }

    pub fn federated_accept<F1, F2>(
        &self,
        voted: F1,
        accepted: F2,
        envs: &HashMap<proto::SCPNodeID, proto::SCPEnvelope>,
    ) -> bool
    where
        F1: Fn(&proto::SCPStatement) -> bool,
        F2: Fn(&proto::SCPStatement) -> bool,
    {
        panic!("XXX");
    }

    pub fn federated_ratify<F>(
        &self,
        voted: F,
        envs: &HashMap<proto::SCPNodeID, proto::SCPEnvelope>,
    ) -> bool
    where
        F: Fn(&proto::SCPStatement) -> bool,
    {
        panic!("XXX")
    }

    pub fn stop_nomination(&mut self) {
        panic!("XXX")
    }
}
