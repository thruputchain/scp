use crate::*;
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;
use tracing::instrument;

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
    pub statement: scp_proto::SCPStatement,
    pub validated: bool,
}

impl SlotState {
    pub fn new(slot_index: SlotIndex, is_validator: bool) -> Self {
        SlotState {
            slot_index,
            nomination: NominationState::new(),
            ballot: BallotState::new(),
            statements_history: Vec::new(),
            fully_validated: is_validator,
            got_vblocking: false,
        }
    }
}

#[derive(Debug)]
pub struct SlotProto<'a, C: Consensus, D: Driver> {
    pub cd: &'a ConsensusDriver<C, D>,
    pub local_state: &'a LocalState,
    pub slot_state: &'a mut SlotState,
}

impl<'a, C: Consensus, D: Driver> SlotProto<'a, C, D> {
    pub fn nomination_proto(&mut self) -> NominationProto<'_, 'a, C, D> {
        NominationProto(self)
    }

    pub fn ballot_proto(&mut self) -> BallotProto<'_, 'a, C, D> {
        BallotProto(self)
    }

    pub fn local_proto(&self) -> LocalProto<'_, C, D> {
        LocalProto {
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

    pub fn driver(&self) -> &D {
        &self.cd.driver
    }

    pub fn consensus(&self) -> &C {
        &self.cd.consensus
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
        statement: &scp_proto::SCPStatement,
    ) -> scp_proto::SCPHash {
        federated::get_companion_quorum_set_hash_from_statement(statement)
    }

    /// returns (nom values, ballot values)
    pub fn get_statement_values(
        statement: &scp_proto::SCPStatement,
    ) -> (Vec<scp_proto::SCPValue>, Vec<scp_proto::SCPValue>) {
        match &statement.pledges {
            scp_proto::SCPStatement_pledges::SCP_ST_NOMINATE { .. } => (
                NominationProto::<C, D>::get_statement_values(statement),
                vec![],
            ),
            _ => {
                let vals = BallotProto::<C, D>::get_statement_values(statement);
                (vec![], vals.into_iter().collect())
            }
        }
    }

    pub fn get_quorum_set_from_statement(
        &self,
        statement: &scp_proto::SCPStatement,
    ) -> Option<scp_proto::SCPQuorumSet> {
        match &statement.pledges {
            scp_proto::SCPStatement_pledges::SCP_ST_EXTERNALIZE { .. } => {
                Some(self.local_proto().get_singleton_qset(&statement.node_id))
            }
            scp_proto::SCPStatement_pledges::SCP_ST_PREPARE { prepare } => {
                self.driver().get_qset(&prepare.quorum_set_hash)
            }
            scp_proto::SCPStatement_pledges::SCP_ST_CONFIRM { confirm } => {
                self.driver().get_qset(&confirm.quorum_set_hash)
            }
            scp_proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate } => {
                self.driver().get_qset(&nominate.quorum_set_hash)
            }
            _ => panic!("unreachable"),
        }
    }

    pub fn get_state(
        &mut self,
        node: &scp_proto::SCPNodeID,
        self_already_moved_on: bool,
    ) -> QuorumInfoNodeState {
        let ballot_state =
            self.ballot_proto().get_state(node, self_already_moved_on);
        if ballot_state != QuorumInfoNodeState::NoInfo {
            return ballot_state;
        }
        self.nomination_proto()
            .get_state(node, self_already_moved_on)
    }

    pub fn should_process_envelope(
        &mut self,
        env: &scp_proto::SCPEnvelope,
        ours: bool,
    ) -> bool {
        assert_eq!(env.statement.slot_index, self.index());
        match &env.statement.pledges {
            scp_proto::SCPStatement_pledges::SCP_ST_NOMINATE { .. } => {
                self.nomination_proto().should_process_envelope(env)
            }
            _ => self.ballot_proto().should_process_envelope(env, ours),
        }
    }

    #[instrument(skip_all)]
    pub fn process_envelope(
        &mut self,
        envelope: &scp_proto::SCPEnvelope,
        ours: bool,
    ) -> EnvelopeState {
        assert_eq!(envelope.statement.slot_index, self.index());

        tracing::trace!(
            envelope = ?envelope,
            slot_index = self.index(),
            "slot proto process_envelope,   i: {}, e: {}",
            self.index(),
            self.cd.envelope_to_str(envelope, false),
        );

        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                match &envelope.statement.pledges {
                    scp_proto::SCPStatement_pledges::SCP_ST_NOMINATE {
                        ..
                    } => self.nomination_proto().process_envelope(envelope),
                    _ => self.ballot_proto().process_envelope(envelope, ours),
                }
            }));

        let result = match result {
            Ok(envelope_state) => envelope_state,
            Err(panic_payload) => {
                let local_node_id = self.local_proto().get_node_id().clone();
                tracing::error!(
                    local_node_id = ?local_node_id,
                    "SCP context {}",
                    self.driver().to_short_string(&local_node_id),
                );
                let json_info = self.get_json_info(false);
                tracing::error!(
                    json_info = ?json_info,
                    "slot JSON info: {}",
                    serde_json::to_string_pretty(&json_info)
                        .unwrap_or_else(|_| "failed to serialize".to_string()),
                );
                tracing::error!(
                    slot_index = self.index(),
                    envelope = ?envelope,
                    "exception processing SCP messages,   i: {}, e: {}",
                    self.index(),
                    self.cd.envelope_to_str(envelope, false),
                );
                tracing::error!("INTERNAL BUG");

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

    pub fn bump_state(
        &mut self,
        value: &scp_proto::SCPValue,
        force: bool,
    ) -> bool {
        self.ballot_proto().bump_state(value.clone(), force)
    }

    pub fn nominate(
        &mut self,
        value: &scp_proto::SCPValue,
        previous_value: &scp_proto::SCPValue,
        timed_out: bool,
    ) -> bool {
        self.nomination_proto()
            .nominate(value, previous_value, timed_out)
    }

    pub fn stop_nomination(&mut self) {
        self.nomination_proto().stop_nomination()
    }

    pub fn get_nomination_leaders(&mut self) -> HashSet<scp_proto::SCPNodeID> {
        self.nomination_proto().get_leaders()
    }

    pub fn set_fully_validated(&mut self, validated: bool) {
        self.slot_state.fully_validated = validated;
    }

    pub fn get_latest_composite_candidate(
        &mut self,
    ) -> Option<scp_proto::SCPValue> {
        self.nomination_proto().get_latest_composite_candidate()
    }

    pub fn create_envelope(
        &mut self,
        statement: scp_proto::SCPStatement,
    ) -> scp_proto::SCPEnvelope {
        use scp_proto::{SCPEd25519Signature, SCPSignature};

        let mut envelope = scp_proto::SCPEnvelope {
            statement,
            signature: SCPSignature::Ed25519 {
                ed25519: SCPEd25519Signature::new([0u8; 64]),
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
        let mut qsets_used: HashMap<
            scp_proto::SCPHash,
            scp_proto::SCPQuorumSet,
        > = HashMap::new();

        for item in self.slot_state.statements_history.iter() {
            let mut statement_array = Vec::new();

            if let Ok(duration) =
                item.when.duration_since(std::time::UNIX_EPOCH)
            {
                statement_array.push(Value::Number(serde_json::Number::from(
                    duration.as_secs(),
                )));
            } else {
                statement_array
                    .push(Value::Number(serde_json::Number::from(0u64)));
            }

            statement_array.push(Value::String(
                self.cd.statement_to_str(&item.statement, full_keys),
            ));
            statement_array.push(Value::Bool(item.validated));

            let qset_hash = Self::get_companion_quorum_set_hash_from_statement(
                &item.statement,
            );
            if let Some(qset) = self.driver().get_qset(&qset_hash) {
                qsets_used.insert(qset_hash, qset.clone());
            }

            statements.push(Value::Array(statement_array));
        }

        let mut quorum_sets_obj = serde_json::Map::new();
        for (hash, qset) in qsets_used {
            let hash_abbrev = hex_abbrev(&hash);
            quorum_sets_obj.insert(
                hash_abbrev,
                self.local_proto().to_json(&qset, full_keys),
            );
        }

        json!({
            "statements": statements,
            "quorum_sets": quorum_sets_obj,
            "validated": self.slot_state.fully_validated,
            "nomination_proto": self.nomination_proto().get_json_info(),
            "ballot_proto": self.ballot_proto().get_json_info()
        })
    }

    pub fn get_json_quorum_info(
        &mut self,
        id: &scp_proto::SCPNodeID,
        summary: bool,
        full_keys: bool,
    ) -> serde_json::Value {
        let mut ret = self
            .ballot_proto()
            .get_json_quorum_info(id, summary, full_keys);

        if self.local_proto().is_validator() {
            ret["validated"] =
                serde_json::Value::Bool(self.is_fully_validated());
        }

        ret
    }

    #[instrument(skip_all)]
    pub fn federated_accept<F1, F2>(
        &self,
        voted: &F1,
        accepted: &F2,
        envs: &HashMap<scp_proto::SCPNodeID, scp_proto::SCPEnvelope>,
    ) -> bool
    where
        F1: Fn(&scp_proto::SCPStatement) -> bool,
        F2: Fn(&scp_proto::SCPStatement) -> bool,
    {
        let local_proto = self.local_proto();
        let qset = local_proto.get_qset();

        // Checks if the nodes that claimed to accept the statement form a v-blocking set
        if LocalProto::<C, D>::is_vblocking_envelopes(qset, envs, accepted) {
            return true;
        }

        // Checks if the set of nodes that accepted or voted for it form a quorum
        let ratify_filter = |st: &scp_proto::SCPStatement| -> bool {
            accepted(st) || voted(st)
        };

        let qset_getter =
            |st: &scp_proto::SCPStatement| -> Option<scp_proto::SCPQuorumSet> {
                self.get_quorum_set_from_statement(st)
            };

        LocalProto::<C, D>::is_quorum(qset, envs, &qset_getter, &ratify_filter)
    }

    #[instrument(skip_all)]
    pub fn federated_ratify<F>(
        &self,
        voted: &F,
        envs: &HashMap<scp_proto::SCPNodeID, scp_proto::SCPEnvelope>,
    ) -> bool
    where
        F: Fn(&scp_proto::SCPStatement) -> bool,
    {
        let local_proto = self.local_proto();
        let qset = local_proto.get_qset();

        let qset_getter =
            |st: &scp_proto::SCPStatement| -> Option<scp_proto::SCPQuorumSet> {
                self.get_quorum_set_from_statement(st)
            };

        LocalProto::<C, D>::is_quorum(qset, envs, &qset_getter, voted)
    }

    pub fn get_statement_count(&self) -> usize {
        self.slot_state.statements_history.len()
    }

    pub fn got_vblocking(&self) -> bool {
        self.slot_state.got_vblocking
    }

    pub fn get_entire_current_state(&mut self) -> Vec<scp_proto::SCPEnvelope> {
        let mut res = Vec::new();
        self.process_current_state(
            &mut |envelope: &scp_proto::SCPEnvelope| {
                res.push(envelope.clone());
                true
            },
            true,
        );
        res
    }

    pub fn get_latest_messages_send(&mut self) -> Vec<scp_proto::SCPEnvelope> {
        let mut res = Vec::new();

        if self.slot_state.fully_validated {
            // Get nomination protocol's last message
            if let Some(nomination_envelope) =
                self.nomination_proto().get_last_message_send()
            {
                res.push(nomination_envelope);
            }

            // Get ballot protocol's last message
            if let Some(ballot_envelope) =
                self.ballot_proto().get_last_message_send()
            {
                res.push(ballot_envelope);
            }
        }

        res
    }

    pub fn get_latest_message(
        &mut self,
        node_id: &scp_proto::SCPNodeID,
    ) -> Option<scp_proto::SCPEnvelope> {
        // Check ballot protocol first
        if let Some(envelope) = self.ballot_proto().get_latest_message(node_id)
        {
            return Some(envelope);
        }
        // Fall back to nomination protocol
        if let Some(envelope) =
            self.nomination_proto().get_latest_message(node_id)
        {
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

        LocalProto::<C, D>::for_all_nodes(&qset, &mut |node_id| {
            if self.get_latest_message(node_id).is_some() {
                nodes.push(node_id.clone());
            }
            true
        });

        self.slot_state.got_vblocking =
            LocalProto::<C, D>::is_vblocking_nodes(&qset, &nodes);

        if self.slot_state.got_vblocking {
            tracing::trace!("got v-blocking for slot {}", self.index());
        }
    }

    pub fn set_state_from_envelope(
        &mut self,
        envelope: &scp_proto::SCPEnvelope,
    ) {
        let statement = &envelope.statement;
        let local_node_id = self.local_proto().get_node_id().clone();

        if statement.node_id == local_node_id
            && statement.slot_index == self.index()
        {
            let had_previous_message =
                self.get_latest_message(&statement.node_id).is_some();

            match &statement.pledges {
                scp_proto::SCPStatement_pledges::SCP_ST_NOMINATE { .. } => {
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
                "slot proto set_state_from_envelope: invalid envelope for slot {} from node {:?}   e: {}",
                self.slot_state.slot_index,
                statement.node_id,
                self.cd.envelope_to_str(envelope, false)
            );
        }
    }

    pub fn process_current_state<F>(&mut self, f: &mut F, force_self: bool)
    where
        F: FnMut(&scp_proto::SCPEnvelope) -> bool,
    {
        if self.nomination_proto().process_current_state(f, force_self) {
            self.ballot_proto().process_current_state(f, force_self);
        }
    }

    pub fn is_newer_nomination_or_ballot_st(
        old_st: &scp_proto::SCPStatement,
        new_st: &scp_proto::SCPStatement,
    ) -> bool {
        let old_nomination = matches!(
            old_st.pledges,
            scp_proto::SCPStatement_pledges::SCP_ST_NOMINATE { .. }
        );
        let new_nomination = matches!(
            new_st.pledges,
            scp_proto::SCPStatement_pledges::SCP_ST_NOMINATE { .. }
        );

        if old_nomination != new_nomination {
            return false;
        }

        // Handle each type appropriately
        if old_nomination {
            if let (
                scp_proto::SCPStatement_pledges::SCP_ST_NOMINATE {
                    nominate: old_nom,
                },
                scp_proto::SCPStatement_pledges::SCP_ST_NOMINATE {
                    nominate: new_nom,
                },
            ) = (&old_st.pledges, &new_st.pledges)
            {
                NominationProto::<C, D>::is_newer_statement(old_nom, new_nom)
            } else {
                panic!("bug")
            }
        } else {
            BallotProto::<C, D>::is_newer_statement(old_st, new_st)
        }
    }

    pub fn get_externalizing_state(&mut self) -> Vec<scp_proto::SCPEnvelope> {
        self.ballot_proto().get_externalizing_state()
    }

    /// Builds a proof snapshot for this slot after it externalizes.
    ///
    /// Returns `Ok(None)` only when the slot has no externalizing state yet.
    pub fn get_externalized_slot_proof(
        &mut self,
    ) -> Result<Option<ExternalizedSlotProof>, ExternalizedSlotProofError> {
        let envelopes = self.get_externalizing_state();
        if envelopes.is_empty() {
            return Ok(None);
        }

        let mut envelopes_by_node = HashMap::with_capacity(envelopes.len());
        let mut companion_qsets = HashMap::new();

        for envelope in envelopes {
            let node_id = envelope.statement.node_id.clone();
            if envelopes_by_node.contains_key(&node_id) {
                return Err(
                    ExternalizedSlotProofError::DuplicateNodeStatement {
                        node_id,
                    },
                );
            }
            envelopes_by_node.insert(node_id, envelope.clone());

            let qset_hash =
                federated::try_get_companion_quorum_set_hash_from_statement(
                    &envelope.statement,
                )
                .map_err(|error| {
                    ExternalizedSlotProofError::MalformedEnvelope {
                        reason: error.into(),
                    }
                })?;
            if companion_qsets.contains_key(&qset_hash) {
                continue;
            }

            let qset = self.driver().get_qset(&qset_hash).ok_or(
                ExternalizedSlotProofError::MissingCompanionQuorumSet {
                    hash: qset_hash,
                },
            )?;
            let actual_hash = scp_util::hash_xdr_scp(&qset).map_err(|_| {
                ExternalizedSlotProofError::CompanionQuorumSetHashFailed {
                    hash: qset_hash,
                }
            })?;
            if actual_hash != qset_hash {
                return Err(
                    ExternalizedSlotProofError::CompanionQuorumSetHashMismatch {
                        claimed: qset_hash,
                        actual: actual_hash,
                    },
                );
            }
            let qset_bytes = scp_util::marshal_xdr(&qset).map_err(|_| {
                ExternalizedSlotProofError::CompanionQuorumSetEncodingFailed {
                    hash: qset_hash,
                }
            })?;
            companion_qsets.insert(qset_hash, qset_bytes);
        }

        Ok(Some(ExternalizedSlotProof::new(
            envelopes_by_node,
            companion_qsets,
        )))
    }

    pub fn record_statement(&mut self, st: &scp_proto::SCPStatement) {
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
            "new statement recorded, i: {} st: {} validated: {}",
            self.slot_state.slot_index,
            self.cd.statement_to_str(st, false),
            self.slot_state.fully_validated,
        );
    }
}
