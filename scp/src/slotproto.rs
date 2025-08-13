use crate::*;

pub type SlotIndex = u64;

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
    pub fn ballot_proto(&'a mut self) -> BallotProto<'a, C, D> {
        crate::ballotproto::BallotProto(self)
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
}
