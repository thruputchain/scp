use crate::*;

pub type SlotIndex = u64;

pub struct SlotState {
    slot_index: SlotIndex,
    ballot_state: BallotState,
}

impl SlotState {
    pub fn new() -> Self {
        SlotState {
            slot_index: 0,
            ballot_state: BallotState::new(),
        }
    }

    pub fn ballot(&self) -> &BallotState {
        &self.ballot_state
    }

    pub fn ballot_mut(&mut self) -> &mut BallotState {
        &mut self.ballot_state
    }

    pub fn slot_index(&self) -> SlotIndex {
        self.slot_index
    }

    pub fn is_statement_sane(&self, statement: &proto::SCPStatement, ours: bool) -> bool {
        panic!("XXX")
    }
}

pub struct SlotProto<'a, C: Consensus, D: Driver> {
    stellar_proto: &'a StellarProto<C, D>,
    slot_state: &'a mut SlotState,
}

impl<'a, C: Consensus, D: Driver> SlotProto<'a, C, D> {
    pub fn new(stellar_proto: &'a mut StellarProto<C, D>, slot_state: &'a mut SlotState) -> Self {
        SlotProto {
            stellar_proto,
            slot_state,
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
}
