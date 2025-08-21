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
    pub when: SystemTime,
    pub statement: proto::SCPStatement,
    pub validated: bool,
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

    pub fn get_companion_quorum_set_hash_from_statement(
        statement: &proto::SCPStatement,
    ) -> proto::SCPHash {
        match &statement.pledges {
            proto::SCPStatement_pledges::SCP_ST_PREPARE { prepare } => {
                prepare.quorum_set_hash.clone()
            }
            proto::SCPStatement_pledges::SCP_ST_CONFIRM { confirm } => {
                confirm.quorum_set_hash.clone()
            }
            proto::SCPStatement_pledges::SCP_ST_EXTERNALIZE { externalize } => {
                externalize.commit_quorum_set_hash.clone()
            }
            proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate } => {
                nominate.quorum_set_hash.clone()
            }
        }
    }

    pub fn get_quorum_set_from_statement(
        &self,
        statement: &proto::SCPStatement,
    ) -> Option<proto::SCPQuorumSet> {
        panic!("XXX")
    }

    pub fn process_envelope(&mut self, envelope: &proto::SCPEnvelope, ours: bool) -> EnvelopeState {
        assert_eq!(envelope.statement.slot_index, self.index());

        tracing::trace!(
            envelope = ?envelope,
            slot_index = self.index(),
            "slot proto process_envelope"
        );

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            match &envelope.statement.pledges {
                proto::SCPStatement_pledges::SCP_ST_NOMINATE { .. } => {
                    self.nomination_proto().process_envelope(envelope)
                }
                _ => self.ballot_proto().process_envelope(envelope, ours),
            }
        }));

        let result = match result {
            Ok(envelope_state) => envelope_state,
            Err(panic_payload) => {
                let local_node_id = self.local_proto().get_node_id().clone();
                tracing::error!(
                    local_node_id = ?local_node_id,
                    "SCP context"
                );
                let json_info = self.get_json_info(false);
                tracing::error!(
                    json_info = %serde_json::to_string_pretty(&json_info).unwrap_or_else(|_| "failed to serialize".to_string()),
                    "slot JSON info"
                );
                tracing::error!(
                    slot_index = self.index(),
                    envelope = ?envelope,
                    "exception processing SCP messages"
                );
                tracing::error!("REPORT_INTERNAL_BUG");

                std::panic::resume_unwind(panic_payload);
            }
        };

        let had_previous_message = self
            .get_latest_message(&envelope.statement.node_id)
            .is_some();
        if !had_previous_message && result == EnvelopeState::Valid {
            self.maybe_set_got_vblocking();
        }

        result
    }

    pub fn abandon_ballot(&mut self) -> bool {
        self.ballot_proto().abandon_ballot(0)
    }

    pub fn bump_state(&mut self, value: &proto::SCPValue, force: bool) -> bool {
        self.ballot_proto().bump_state(value.clone(), force)
    }

    pub fn nominate(
        &mut self,
        value: &proto::SCPValue,
        previous_value: &proto::SCPValue,
        timed_out: bool,
    ) -> bool {
        self.nomination_proto()
            .nominate(value, previous_value, timed_out)
    }

    pub fn stop_nomination(&mut self) {
        self.nomination_proto().stop_nomination()
    }

    pub fn get_nomination_leaders(&mut self) -> HashSet<proto::SCPNodeID> {
        self.nomination_proto().get_leaders()
    }

    pub fn set_fully_validated(&mut self, validated: bool) {
        self.slot_state.fully_validated = validated;
    }

    pub fn get_latest_composite_candidate(&mut self) -> Option<proto::SCPValue> {
        self.nomination_proto().get_latest_composite_candidate()
    }

    pub fn create_envelope(&mut self, statement: proto::SCPStatement) -> proto::SCPEnvelope {
        use proto::{Ed25519Signature, Signature};

        let mut envelope = proto::SCPEnvelope {
            statement: statement,
            signature: Signature::Ed25519 {
                ed25519: Ed25519Signature::new([0u8; 64]),
            },
        };

        envelope.statement.node_id = self.local_proto().get_node_id().clone();
        envelope.statement.slot_index = self.index();

        self.driver().sign_envelope(&mut envelope);

        envelope
    }

    pub fn is_fully_validated(&self) -> bool {
        self.slot_state.fully_validated
    }

    pub fn get_json_info(&mut self, full_keys: bool) -> serde_json::Value {
        use serde_json::{Value, json};

        let mut statements = Vec::new();
        let mut qsets_used: HashMap<proto::SCPHash, proto::SCPQuorumSet> = HashMap::new();

        for item in self.slot_state.statements_history.iter() {
            let mut statement_array = Vec::new();

            if let Ok(duration) = item.when.duration_since(std::time::UNIX_EPOCH) {
                statement_array.push(Value::Number(serde_json::Number::from(duration.as_secs())));
            } else {
                statement_array.push(Value::Number(serde_json::Number::from(0u64)));
            }

            statement_array.push(Value::String(
                self.cd.statement_to_str(&item.statement, full_keys),
            ));
            statement_array.push(Value::Bool(item.validated));

            let qset_hash = Self::get_companion_quorum_set_hash_from_statement(&item.statement);
            if let Some(qset) = self.driver().get_qset(&qset_hash) {
                qsets_used.insert(qset_hash, qset.clone());
            }

            statements.push(Value::Array(statement_array));
        }

        let mut quorum_sets_obj = serde_json::Map::new();
        for (hash, qset) in qsets_used {
            let hash_abbrev = hex_abbrev(&hash);
            quorum_sets_obj.insert(hash_abbrev, self.local_proto().to_json(&qset, full_keys));
        }

        json!({
            "statements": statements,
            "quorum_sets": quorum_sets_obj,
            "validated": self.slot_state.fully_validated,
            "nomination_proto": self.nomination_proto().get_json_info(),
            "ballot_proto": self.ballot_proto().get_json_info()
        })
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

    pub fn get_externalizing_state(&mut self) -> Vec<proto::SCPEnvelope> {
        self.ballot_proto().get_externalizing_state()
    }

    pub fn record_statement(&mut self, st: &proto::SCPStatement) {
        let historical_statement = HistoricalStatement {
            when: std::time::SystemTime::now(), //XXX:diff
            statement: st.clone(),
            validated: self.slot_state.fully_validated,
        };

        self.slot_state
            .statements_history
            .push(historical_statement);

        // Log the statement (similar to C++ version)
        tracing::debug!(
            statement = ?st,
            validated = self.slot_state.fully_validated,
            slot_index = self.slot_state.slot_index,
            "new statement recorded"
        );
    }
}
