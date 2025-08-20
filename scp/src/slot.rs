use crate::*;
use std::collections::HashMap;
use std::time::SystemTime;

pub type SlotIndex = u64;

// The number of times the timer has to expire before we consider the node
// missing. Used for reporting purposes only.
pub const NUM_TIMEOUTS_THRESHOLD_FOR_REPORTING: i64 = 2;

#[derive(Debug)]
pub struct SlotState {
    pub slot_index: SlotIndex,
    pub nomination: NominationState,
    pub ballot: BallotState,
    pub statements_history: Vec<HistoricalStatement>,
    pub fully_validated: bool,
    pub got_vblocking: bool,
}

#[derive(Debug, Clone)]
pub struct HistoricalStatement {
    when: SystemTime,
    statement: proto::SCPStatement,
    validated: bool,
}

impl SlotState {
    pub fn new() -> Self {
        SlotState {
            slot_index: 0,
            nomination: NominationState::new(),
            ballot: BallotState::new(),
            statements_history: Vec::new(),
            fully_validated: false, //XXX
            got_vblocking: false,
        }
    }
}

#[derive(Debug)]
pub struct SlotProto<'a> {
    pub cd: &'a ConsensusDriver,
    pub local_state: &'a mut LocalState,
    pub slot_state: &'a mut SlotState,
}

impl<'a> SlotProto<'a> {
    //XXX: call this when creating a slot
    pub fn init(&mut self) {
        self.slot_state.fully_validated = self.local_proto().is_validator()
    }

    pub fn nomination_proto(&mut self) -> NominationProto<'_, 'a> {
        NominationProto(self)
    }

    pub fn ballot_proto(&mut self) -> BallotProto<'_, 'a> {
        BallotProto(self)
    }

    pub fn local_proto(&self) -> LocalProto<'_> {
        LocalProto {
            cd: self.cd,
            local: self.local_state,
        }
    }

    pub fn local_proto_mut(&mut self) -> LocalProtoMut<'_> {
        LocalProtoMut {
            cd: self.cd,
            local: self.local_state,
        }
    }

    pub fn index(&self) -> SlotIndex {
        self.slot_state.slot_index
    }

    pub fn ballot_state(&self) -> &BallotState {
        &self.slot_state.ballot
    }

    pub fn ballot_state_mut(&mut self) -> &mut BallotState {
        &mut self.slot_state.ballot
    }

    pub fn nomination(&self) -> &NominationState {
        &self.slot_state.nomination
    }

    pub fn nomination_mut(&mut self) -> &mut NominationState {
        &mut self.slot_state.nomination
    }

    pub fn driver(&self) -> &dyn Driver {
        self.cd.driver.as_ref()
    }

    pub fn consensus(&self) -> &dyn Consensus {
        self.cd.consensus.as_ref()
    }

    pub fn driver_calling_back(&mut self, cb: DriverCallback) {
        match cb {
            DriverCallback::BallotTimerExpired { slot_index: _ } => {
                self.ballot_proto().driver_calling_back(cb)
            }
            DriverCallback::NominationTimerExpired {
                slot_index: _,
                value: _,
                prev_value: _,
            } => self.nomination_proto().driver_calling_back(cb),
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
