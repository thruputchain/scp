use crate::*;
use std::collections::HashMap;

pub type SlotIndex = u64;

// The number of times the timer has to expire before we consider the node
// missing. Used for reporting purposes only.
pub const NUM_TIMEOUTS_THRESHOLD_FOR_REPORTING: i64 = 2;

#[derive(Debug)]
pub struct Slot {
    pub slot_index: SlotIndex,
    pub nomination: NominationState,
    pub ballot: BallotState,
}

impl Slot {
    pub fn new() -> Self {
        Slot {
            slot_index: 0,
            nomination: NominationState::new(),
            ballot: BallotState::new(),
        }
    }
}

#[derive(Debug)]
pub struct SlotProto<'a> {
    pub proxy: &'a StellarProto<'a>,
    pub cd: &'a ConsensusDriver,
    pub local: &'a mut LocalState,
    pub slot: &'a mut Slot,
}

impl<'a> SlotProto<'a> {
    pub fn ballot_proto(&mut self) -> BallotProto<'_, 'a> {
        BallotProto(self)
    }

    pub fn local_proto(&self) -> LocalProto<'_> {
        LocalProto {
            cd: self.cd,
            local: self.local,
        }
    }

    pub fn local_proto_mut(&mut self) -> LocalProtoMut<'_> {
        LocalProtoMut {
            cd: self.cd,
            local: self.local,
        }
    }

    pub fn index(&self) -> SlotIndex {
        self.slot.slot_index
    }

    pub fn ballot(&self) -> &BallotState {
        &self.slot.ballot
    }

    pub fn ballot_mut(&mut self) -> &mut BallotState {
        &mut self.slot.ballot
    }

    pub fn nomination(&self) -> &NominationState {
        &self.slot.nomination
    }

    pub fn nomination_mut(&mut self) -> &mut NominationState {
        &mut self.slot.nomination
    }

    pub fn driver(&self) -> &dyn Driver {
        self.cd.driver.as_ref()
    }

    pub fn consensus(&self) -> &dyn Consensus {
        self.cd.consensus.as_ref()
    }

    pub fn timer_expired(&mut self, timer: Timer) {
        match timer {
            Timer::Ballot => self.ballot_proto().timer_expired(),
            Timer::Nomination => panic!("XXX"),
        }
    }

    pub fn get_quorum_set_from_statement(
        &self,
        statement: &proto::SCPStatement,
    ) -> Option<proto::SCPQuorumSet> {
        panic!("XXX")
    }

    pub fn record_envelope(&mut self, env: &proto::SCPEnvelope) {
        panic!("XXX")
    }

    pub fn process_envelope(&mut self, envelope: &proto::SCPEnvelope, ours: bool) -> EnvelopeState {
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

    pub fn bump_state(&mut self, value: &proto::SCPValue, force: bool) -> bool {
        panic!("XXX")
    }
}
