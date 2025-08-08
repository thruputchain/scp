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

pub struct SlotProto<'a> {
    slot_state: &'a mut SlotState,
}

impl<'a> SlotProto<'a> {
    pub fn new(slot_state: &'a mut SlotState) -> Self {
        SlotProto { slot_state }
    }

    fn state(&self) -> &SlotState {
        self.slot_state
    }

    fn state_mut(&mut self) -> &mut SlotState {
        self.slot_state
    }

    pub fn record_envelope(&mut self, env: &proto::SCPEnvelope) {
        panic!("XXX")
    }
}
