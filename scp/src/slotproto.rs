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

    pub fn timer_expired_cb(&mut self, timer: Timer) {
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
}
