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
    pub fn slot_index(&self) -> SlotIndex {
        self.slot.slot_index
    }

    pub fn get_qset_from_statement(
        &self,
        statement: &proto::SCPStatement,
    ) -> Option<proto::SCPQuorumSet> {
        panic!("XXX")
    }

    pub fn slot_record_envelope(&mut self, env: &proto::SCPEnvelope) {
        panic!("XXX")
    }

    pub fn set_fully_validated(&mut self, validated: bool) {
        panic!("XXX")
    }

    pub fn get_latest_composite_candidate(&self) -> Option<&proto::SCPValue> {
        panic!("XXX")
    }
}
