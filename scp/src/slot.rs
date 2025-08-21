use crate::*;
use std::collections::{HashMap, HashSet};
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

    pub fn get_latest_composite_candidate(&mut self) -> Option<proto::SCPValue> {
        self.nomination_proto().get_latest_composite_candidate()
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

    pub fn get_statement_count(&self) -> usize {
        self.slot_state.statements_history.len()
    }

    pub fn got_vblocking(&self) -> bool {
        self.slot_state.got_vblocking
    }

    pub fn get_latest_messages_send(&mut self) -> Vec<proto::SCPEnvelope> {
        let mut res = Vec::new();

        if self.slot_state.fully_validated {
            // Get nomination protocol's last message
            if let Some(nomination_envelope) = self.nomination_proto().get_last_message_send() {
                res.push(nomination_envelope);
            }

            // Get ballot protocol's last message
            if let Some(ballot_envelope) = self.ballot_proto().get_last_message_send() {
                res.push(ballot_envelope);
            }
        }

        res
    }

    pub fn get_latest_message(&mut self, node_id: &proto::SCPNodeID) -> Option<proto::SCPEnvelope> {
        // Check ballot protocol first
        if let Some(envelope) = self.ballot_proto().get_latest_message(node_id) {
            return Some(envelope);
        }
        // Fall back to nomination protocol
        if let Some(envelope) = self.nomination_proto().get_latest_message(node_id) {
            return Some(envelope);
        }
        None
    }

    pub fn maybe_set_got_vblocking(&mut self) {
        if self.slot_state.got_vblocking {
            // Already set
            return;
        }

        let qset = self.local_proto().get_qset().clone();
        let mut nodes = Vec::new();

        LocalProto::for_all_nodes(&qset, &mut |node_id| {
            if self.get_latest_message(node_id).is_some() {
                nodes.push(node_id.clone());
            }
            true
        });

        self.slot_state.got_vblocking = LocalProto::is_vblocking_nodes(&qset, &nodes);

        if self.slot_state.got_vblocking {
            tracing::trace!("got v-blocking for slot {}", self.index());
        }
    }

    pub fn set_state_from_envelope(&mut self, envelope: &proto::SCPEnvelope) {
        let statement = &envelope.statement;
        let local_node_id = self.local_proto().get_node_id().clone();

        if statement.node_id == local_node_id && statement.slot_index == self.index() {
            let had_previous_message = self.get_latest_message(&statement.node_id).is_some();

            match &statement.pledges {
                proto::SCPStatement_pledges::SCP_ST_NOMINATE { .. } => {
                    self.nomination_proto().set_state_from_envelope(envelope);
                }
                _ => {
                    self.ballot_proto().set_state_from_envelope(envelope);
                }
            }

            if !had_previous_message {
                self.maybe_set_got_vblocking();
            }
        } else {
            tracing::trace!(
                env = ?envelope,
                "slot proto set_state_from_envelope: invalid envelope for slot {} from node {:?}",
                self.slot_state.slot_index,
                statement.node_id
            );
        }
    }

    pub fn process_current_state<F>(&mut self, f: &mut F, force_self: bool) -> bool
    where
        F: FnMut(&proto::SCPEnvelope) -> bool,
    {
        self.nomination_proto().process_current_state(f, force_self)
            && self.ballot_proto().process_current_state(f, force_self)
    }

    pub fn is_newer_nomination_or_ballot_st(
        old_st: &proto::SCPStatement,
        new_st: &proto::SCPStatement,
    ) -> bool {
        let old_nomination = matches!(
            old_st.pledges,
            proto::SCPStatement_pledges::SCP_ST_NOMINATE { .. }
        );
        let new_nomination = matches!(
            new_st.pledges,
            proto::SCPStatement_pledges::SCP_ST_NOMINATE { .. }
        );

        if old_nomination != new_nomination {
            return false;
        }

        // Handle each type appropriately
        if old_nomination {
            if let (
                proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate: old_nom },
                proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate: new_nom },
            ) = (&old_st.pledges, &new_st.pledges)
            {
                NominationProto::is_newer_statement(old_nom, new_nom)
            } else {
                panic!("bug")
            }
        } else {
            BallotProto::is_newer_statement(old_st, new_st)
        }
    }
}
