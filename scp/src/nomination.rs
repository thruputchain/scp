use crate::*;
use std::collections::{HashMap, HashSet};
use tracing::{Level, debug, enabled, error, instrument, trace};
use xdr_common::VecM;

#[derive(Debug)]
pub struct NominationState {
    round_number: proto::SCPCount,
    votes: HashSet<proto::SCPValue>,      // X
    accepted: HashSet<proto::SCPValue>,   // Y
    candidates: HashSet<proto::SCPValue>, // Z
    latest_nominations: HashMap<proto::SCPNodeID, proto::SCPEnvelope>, // N
    last_envelope: Option<proto::SCPEnvelope>, // last envelope emitted by this node
    round_leaders: HashSet<proto::SCPNodeID>, // nodes from quorum set that have the highest priority this round
    nomination_started: bool,                 // true if 'nominate' was called
    latest_composite_candidate: Option<proto::SCPValue>, // the latest (if any) candidate value
    previous_value: Option<proto::SCPValue>,  // the value from the previous slot
    timer_exp_count: i32, // the number of times the timer has expired; used for the quorum endpoint
}

impl NominationState {
    pub fn new() -> Self {
        NominationState {
            round_number: 0,
            votes: HashSet::new(),
            accepted: HashSet::new(),
            candidates: HashSet::new(),
            latest_nominations: HashMap::new(),
            last_envelope: None,
            round_leaders: HashSet::new(),
            nomination_started: false,
            latest_composite_candidate: None,
            previous_value: None,
            timer_exp_count: 0,
        }
    }
}

#[derive(Debug)]
pub struct NominationProto<'a, 'b>(pub &'a mut SlotProto<'b>);

impl<'a, 'b> NominationProto<'a, 'b> {
    fn slot_mut(&mut self) -> &mut SlotProto<'b> {
        self.0
    }

    fn slot(&self) -> &SlotProto<'b> {
        self.0
    }

    fn nomination(&self) -> &NominationState {
        self.slot().nomination()
    }

    fn nomination_mut(&mut self) -> &mut NominationState {
        self.slot_mut().nomination_mut()
    }

    fn driver(&self) -> &dyn Driver {
        self.slot().driver()
    }

    fn consensus(&self) -> &dyn Consensus {
        self.slot().consensus()
    }

    pub fn is_newer_node_statement(
        &self,
        node_id: &proto::SCPNodeID,
        st: &proto::SCPStatementNominate,
    ) -> bool {
        match self.nomination().latest_nominations.get(node_id) {
            None => true,
            Some(old_envelope) => {
                if let proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate: old_st } =
                    &old_envelope.statement.pledges
                {
                    Self::is_newer_statement(old_st, st)
                } else {
                    false
                }
            }
        }
    }

    pub fn is_newer_statement(
        oldst: &proto::SCPStatementNominate,
        st: &proto::SCPStatementNominate,
    ) -> bool {
        let mut grows_votes = false;

        if Self::is_subset(&oldst.votes, &st.votes, &mut grows_votes) {
            let mut grows_accepted = false;
            if Self::is_subset(&oldst.accepted, &st.accepted, &mut grows_accepted) {
                grows_votes || grows_accepted
            } else {
                false
            }
        } else {
            false
        }
    }

    fn is_subset(
        p: &VecM<proto::SCPValue>,
        v: &VecM<proto::SCPValue>,
        not_equal: &mut bool,
    ) -> bool {
        if p.len() <= v.len() {
            let mut p_sorted: Vec<_> = p.iter().cloned().collect();
            let mut v_sorted: Vec<_> = v.iter().cloned().collect();
            p_sorted.sort();
            v_sorted.sort();

            let is_subset = p_sorted
                .iter()
                .all(|item| v_sorted.binary_search(item).is_ok());
            if is_subset {
                *not_equal = p.len() != v.len();
            } else {
                *not_equal = true;
            }
            is_subset
        } else {
            *not_equal = true;
            false
        }
    }

    pub fn is_sane(&self, st: &proto::SCPStatement) -> bool {
        if let proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate: nom } = &st.pledges {
            // Check that nomination has at least one vote or accepted value
            let has_content = !nom.votes.is_empty() || !nom.accepted.is_empty();

            if !has_content {
                return false;
            }

            // Check that votes are sorted
            let votes_sorted = Self::is_sorted(&nom.votes);

            // Check that accepted values are sorted
            let accepted_sorted = Self::is_sorted(&nom.accepted);

            votes_sorted && accepted_sorted
        } else {
            panic!("bug")
        }
    }

    fn is_sorted(values: &VecM<proto::SCPValue>) -> bool {
        if values.len() <= 1 {
            return true;
        }

        let values_vec: Vec<_> = values.iter().collect();
        values_vec.windows(2).all(|window| window[0] <= window[1])
    }

    pub fn record_envelope(&mut self, env: &proto::SCPEnvelope) {
        let node_id = &env.statement.node_id;
        self.nomination_mut()
            .latest_nominations
            .insert(node_id.clone(), env.clone());
        self.slot_mut().record_statement(&env.statement);
    }

    #[instrument]
    pub fn emit_nomination(&mut self) {
        // Create SCP statement for nomination
        let node_id = self.slot().local_proto().get_node_id().clone();
        let qset_hash = *self.slot().local_proto().get_qset_hash();

        // Create the nomination statement
        let mut votes = VecM(vec![]);
        let mut accepted = VecM(vec![]);

        // Copy votes from nomination state
        for vote in &self.nomination().votes {
            votes.0.push(vote.clone());
        }

        // Copy accepted values from nomination state
        for accept in &self.nomination().accepted {
            accepted.0.push(accept.clone());
        }

        // Sort the collections to ensure they're ordered
        votes.0.sort();
        accepted.0.sort();

        let nominate = proto::SCPStatementNominate {
            quorum_set_hash: qset_hash,
            votes,
            accepted,
        };

        let statement = proto::SCPStatement {
            node_id,
            slot_index: self.slot().index(),
            pledges: proto::SCPStatement_pledges::SCP_ST_NOMINATE {
                nominate: nominate.clone(),
            },
        };

        // Create envelope
        let envelope = self.slot().create_envelope(statement.clone());

        // Process the envelope first - this is crucial!
        match self.slot_mut().process_envelope(&envelope, true) {
            EnvelopeState::Valid => {
                // Check if this statement is newer than our last envelope
                let should_update = match &self.nomination().last_envelope {
                    None => true,
                    Some(last_env) => {
                        if let proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate: last_nom } =
                            &last_env.statement.pledges
                        {
                            Self::is_newer_statement(last_nom, &nominate)
                        } else {
                            true
                        }
                    }
                };

                if should_update {
                    self.nomination_mut().last_envelope = Some(envelope.clone());

                    if self.slot().is_fully_validated() {
                        self.driver().emit_envelope(envelope);
                    }
                }
            }
            EnvelopeState::Invalid =>
            // There is a bug in the application if it queued up
            // a statement for itself that it considers invalid
            {
                panic!("moved to a bad state (nomination)")
            }
        }
    }

    pub fn accept_predicate(v: &proto::SCPValue, st: &proto::SCPStatement) -> bool {
        if let proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate: nom } = &st.pledges {
            nom.accepted.iter().any(|accepted_val| accepted_val == v)
        } else {
            false
        }
    }

    pub fn apply_all<F>(nom: &proto::SCPStatementNominate, mut processor: F)
    where
        F: FnMut(&proto::SCPValue),
    {
        // Apply processor to all votes
        for v in nom.votes.iter() {
            processor(v);
        }

        // Apply processor to accepted values that are not already in votes
        // NB: "accepted" should be a subset of "votes", so this should mostly no-op
        for a in nom.accepted.iter() {
            if !nom.votes.iter().any(|vote| vote == a) {
                processor(a);
            }
        }
    }

    #[instrument]
    pub fn update_round_leaders(&mut self) {
        let mut my_qset = self.slot().local_proto().get_qset().clone();
        let local_id = self.slot().local_proto().get_node_id().clone();

        // Normalize qset (excludes self)
        crate::qset::normalize_qset(&mut my_qset, Some(&local_id));

        // Count maximum leader count (includes self)
        let mut max_leader_count = 1;
        // note that node IDs here are unique ("sane"), so we can count by
        // enumeration
        LocalProto::for_all_nodes(&my_qset, &mut |_| {
            max_leader_count += 1;
            true
        });

        while self.nomination().round_leaders.len() < max_leader_count {
            // Initialize priority with value derived from self
            let mut new_round_leaders = HashSet::new();

            new_round_leaders.insert(local_id.clone());
            let mut top_priority = self.get_node_priority(&local_id, &my_qset);

            // Check all nodes in quorum set for higher priority
            LocalProto::for_all_nodes(&my_qset, &mut |cur| {
                let weight = self.get_node_priority(cur, &my_qset);
                if weight > top_priority {
                    top_priority = weight;
                    new_round_leaders.clear();
                }
                if weight == top_priority && weight > 0 {
                    new_round_leaders.insert(cur.clone());
                }
                true
            });

            if top_priority == 0 {
                // No one had priority, so all nodes would choose themselves
                // resulting in a timeout. Clear newRoundLeaders, allowing the
                // algorithm to fast timeout and try again.
                new_round_leaders.clear();
            }

            // Expand round_leaders with the newly computed leaders
            let old_size = self.nomination().round_leaders.len();

            // Insert new leaders into round_leaders
            for leader in new_round_leaders {
                self.nomination_mut().round_leaders.insert(leader);
            }

            let new_size = self.nomination().round_leaders.len();

            if old_size != new_size {
                if enabled!(Level::DEBUG) {
                    debug!(
                        "nomination update_round_leaders: {} -> {}",
                        old_size, new_size
                    );
                    for rl in &self.nomination().round_leaders {
                        debug!("  leader {}", self.driver().to_short_string(rl));
                    }
                }
                return;
            } else {
                self.nomination_mut().round_number += 1;
                debug!(
                    "nomination update_round_leaders fast timeout (would no op) -> {}",
                    self.nomination().round_number
                );
            }
        }
        debug!("nomination update_round_leaders: nothing to do")
    }

    #[instrument]
    fn get_node_priority(&self, node_id: &proto::SCPNodeID, qset: &proto::SCPQuorumSet) -> u64 {
        let is_local = node_id == self.slot().local_proto().get_node_id();
        let weight = self.driver().get_node_weight(node_id, qset, is_local);

        // if weight > 0; weight is inclusive here as
        // 0 <= hash_node <= u64::MAX
        if weight > 0 && self.hash_node(false, node_id) <= weight {
            self.hash_node(true, node_id)
        } else {
            0
        }
    }

    #[instrument]
    fn hash_node(&self, is_priority: bool, node_id: &proto::SCPNodeID) -> u64 {
        if let Some(ref previous_value) = self.nomination().previous_value {
            self.driver().compute_hash_node(
                self.slot().index(),
                previous_value,
                is_priority,
                self.nomination().round_number,
                node_id,
            )
        } else {
            panic!("bug")
        }
    }

    #[instrument]
    fn hash_value(&self, value: &proto::SCPValue) -> u64 {
        if let Some(ref previous_value) = self.nomination().previous_value {
            self.driver().compute_value_hash(
                self.slot().index(),
                previous_value,
                self.nomination().round_number,
                value,
            )
        } else {
            panic!("bug")
        }
    }

    #[instrument]
    pub fn get_new_value_from_nomination(
        &self,
        nom: &proto::SCPStatementNominate,
    ) -> Option<proto::SCPValue> {
        // Pick the highest value we don't have from the nomination
        // sorted using hash_value.
        let mut new_vote: Option<proto::SCPValue> = None;
        let mut new_hash = 0u64;
        let mut found_valid_value = false;

        let pick_value = |value: &proto::SCPValue,
                          new_vote: &mut Option<proto::SCPValue>,
                          new_hash: &mut u64,
                          found_valid_value: &mut bool| {
            let value_to_nominate = match self.validate_value(value) {
                ValidationLevel::FullyValid => Some(value.clone()),
                _ => self.extract_valid_value(value),
            };

            if let Some(valid_val) = value_to_nominate {
                *found_valid_value = true;
                if !self.nomination().votes.contains(&valid_val) {
                    let cur_hash = self.hash_value(&valid_val);
                    if cur_hash >= *new_hash {
                        *new_hash = cur_hash;
                        *new_vote = Some(valid_val);
                    }
                }
            }
        };

        // First look through accepted values
        for val in nom.accepted.iter() {
            pick_value(val, &mut new_vote, &mut new_hash, &mut found_valid_value);
        }

        // Move on to votes if we have not found a valid accepted value
        if !found_valid_value {
            for val in nom.votes.iter() {
                pick_value(val, &mut new_vote, &mut new_hash, &mut found_valid_value);
            }
        }

        new_vote
    }

    #[instrument]
    fn validate_value(&self, value: &proto::SCPValue) -> ValidationLevel {
        self.consensus()
            .validate_nomination_value(self.slot().index(), value)
    }

    #[instrument]
    fn extract_valid_value(&self, value: &proto::SCPValue) -> Option<proto::SCPValue> {
        self.consensus()
            .extract_valid_value(self.slot().index(), value)
    }

    #[instrument]
    pub fn process_envelope(&mut self, envelope: &proto::SCPEnvelope) -> EnvelopeState {
        let st = &envelope.statement;

        if let proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate: nom } = &st.pledges {
            if !self.is_newer_node_statement(&st.node_id, nom) {
                return EnvelopeState::Invalid;
            }

            if !self.is_sane(st) {
                trace!("nomination proto message didn't pass sanity check");
                return EnvelopeState::Invalid;
            }

            self.record_envelope(envelope);

            if self.nomination().nomination_started {
                // Track if we should emit a new nomination message
                let mut modified = false;
                let mut new_candidates = false;

                // Attempt to promote some of the votes to accepted
                for v in nom.votes.iter() {
                    if self.nomination().accepted.contains(v) {
                        // v is already accepted
                        continue;
                    }

                    // Check if this value can be accepted via federated accept
                    let votes_predicate = |st: &proto::SCPStatement| -> bool {
                        if let proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate: nom } =
                            &st.pledges
                        {
                            nom.votes.iter().any(|vote| vote == v)
                        } else {
                            false
                        }
                    };

                    let accept_predicate =
                        |st: &proto::SCPStatement| -> bool { Self::accept_predicate(v, st) };

                    if self.slot().federated_accept(
                        votes_predicate,
                        accept_predicate,
                        &self.nomination().latest_nominations,
                    ) {
                        let validation_level = self.validate_value(v);
                        if validation_level == ValidationLevel::FullyValid {
                            self.nomination_mut().accepted.insert(v.clone());
                            self.nomination_mut().votes.insert(v.clone());
                            modified = true;
                        } else {
                            // The value made it pretty far: see if we can vote for a variation
                            // that we consider valid
                            if let Some(valid_value) = self.extract_valid_value(v) {
                                if self.nomination_mut().votes.insert(valid_value) {
                                    modified = true;
                                }
                            }
                        }
                    }
                }

                // Attempt to promote accepted values to candidates
                for a in self.nomination().accepted.clone().iter() {
                    if self.nomination().candidates.contains(a) {
                        continue;
                    }

                    let accept_predicate =
                        |st: &proto::SCPStatement| -> bool { Self::accept_predicate(a, st) };

                    if self
                        .slot()
                        .federated_ratify(accept_predicate, &self.nomination().latest_nominations)
                    {
                        self.nomination_mut().candidates.insert(a.clone());
                        new_candidates = true;
                        // Stop the timer, as there's no need to continue nominating
                        self.driver()
                            .stop_timer(self.slot().index(), Timer::Nomination);
                    }
                }

                // Only take round leader votes if we're still looking for candidates
                if self.nomination().candidates.is_empty()
                    && self.nomination().round_leaders.contains(&st.node_id)
                {
                    if let Some(new_vote) = self.get_new_value_from_nomination(nom) {
                        if self.nomination_mut().votes.insert(new_vote.clone()) {
                            modified = true;
                            self.consensus()
                                .nominating_value(self.slot().index(), &new_vote);
                        } else {
                            panic!("bug")
                        }
                    }
                }

                if modified {
                    self.emit_nomination();
                }

                if new_candidates {
                    // Combine candidates and update state
                    if let Some(composite) = self
                        .consensus()
                        .combine_candidates(self.slot().index(), &self.nomination().candidates)
                    {
                        self.nomination_mut().latest_composite_candidate = Some(composite.clone());
                        self.consensus()
                            .updated_candidate_value(self.slot().index(), &composite);
                        self.slot_mut().bump_state(&composite, false);
                    }
                }
            }

            EnvelopeState::Valid
        } else {
            // Not a nomination statement
            EnvelopeState::Invalid
        }
    }

    pub fn get_statement_values(&self, st: &proto::SCPStatement) -> Vec<proto::SCPValue> {
        let mut res = Vec::new();
        if let proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate: nom } = &st.pledges {
            Self::apply_all(nom, |v| {
                res.push(v.clone());
            });
        }
        res
    }

    #[instrument]
    pub fn nominate(
        &mut self,
        value: &proto::SCPValue,
        previous_value: &proto::SCPValue,
        timed_out: bool,
    ) -> bool {
        // No need to continue nominating, as per the whitepaper:
        // "As soon as `v` has a candidate value, however, it must cease
        // voting to nominate `x` for any new values `x`"
        if !self.nomination().candidates.is_empty() {
            debug!(
                "nomination skip nomination round {}, already have a candidate",
                self.nomination().round_number
            );
            return false;
        }

        debug!(
            "nomination nominate ({}) value: {:?}",
            self.nomination().round_number,
            value
        );

        let mut updated = false;

        if timed_out {
            self.nomination_mut().timer_exp_count += 1;
        }

        if timed_out && !self.nomination().nomination_started {
            debug!("nomination nominate (TIMED OUT)");
            return false;
        }

        self.nomination_mut().nomination_started = true;
        self.nomination_mut().previous_value = Some(previous_value.clone());
        self.nomination_mut().round_number += 1;

        self.update_round_leaders();

        let timeout = self
            .driver()
            .compute_timeout(self.nomination().round_number, true);

        // Add a few more values from other leaders
        for leader in self.nomination().round_leaders.clone().iter() {
            if let Some(latest_nomination) = self.nomination().latest_nominations.get(leader) {
                if let proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate: nom } =
                    &latest_nomination.statement.pledges
                {
                    if let Some(new_vote) = self.get_new_value_from_nomination(nom) {
                        if self.nomination_mut().votes.insert(new_vote.clone()) {
                            updated = true;
                            self.consensus()
                                .nominating_value(self.slot().index(), &new_vote);
                        }
                    }
                }
            }
        }

        // If we're leader, add our value if we haven't added any votes yet
        let local_node_id = self.slot().local_proto().get_node_id().clone();
        if self.nomination().round_leaders.contains(&local_node_id)
            && self.nomination().votes.is_empty()
        {
            if self.nomination_mut().votes.insert(value.clone()) {
                updated = true;
                self.consensus()
                    .nominating_value(self.slot().index(), value);
            }
        }

        // Setup timer for next nomination round
        let slot_index = self.slot().index();
        self.driver().start_timer(
            slot_index,
            Timer::Nomination,
            timeout,
            DriverCallback::NominationTimerExpired {
                slot_index: slot_index,
                value: value.clone(),
                prev_value: previous_value.clone(),
            },
        );

        if updated {
            self.emit_nomination();
        } else {
            debug!("nomination nominate (SKIPPED)");
        }

        updated
    }

    pub fn driver_calling_back(&mut self, cb: DriverCallback) {
        match cb {
            DriverCallback::NominationTimerExpired {
                slot_index: _,
                value,
                prev_value,
            } => {
                self.nominate(&value, &prev_value, true);
            }
            _ => {}
        }
    }

    pub fn stop_nomination(&mut self) {
        self.nomination_mut().nomination_started = false;
    }

    pub fn get_leaders(&self) -> HashSet<proto::SCPNodeID> {
        self.nomination().round_leaders.clone()
    }

    pub fn get_json_info(&self) -> serde_json::Value {
        let mut ret = serde_json::json!({});

        ret["roundnumber"] = serde_json::Value::from(self.nomination().round_number);
        ret["started"] = serde_json::Value::from(self.nomination().nomination_started);

        let mut votes_array = Vec::new();
        for vote in &self.nomination().votes {
            votes_array.push(serde_json::Value::String(
                self.driver().get_value_string(vote),
            ));
        }
        ret["X"] = serde_json::Value::Array(votes_array);

        let mut accepted_array = Vec::new();
        for accepted in &self.nomination().accepted {
            accepted_array.push(serde_json::Value::String(
                self.driver().get_value_string(accepted),
            ));
        }
        ret["Y"] = serde_json::Value::Array(accepted_array);

        let mut candidates_array = Vec::new();
        for candidate in &self.nomination().candidates {
            candidates_array.push(serde_json::Value::String(
                self.driver().get_value_string(candidate),
            ));
        }
        ret["Z"] = serde_json::Value::Array(candidates_array);

        ret
    }

    pub fn get_state(
        &self,
        node_id: &proto::SCPNodeID,
        self_already_moved_on: bool,
    ) -> QuorumInfoNodeState {
        let mut state = QuorumInfoNodeState::Agree;

        if node_id == self.slot().local_proto().get_node_id() {
            // always mark myself as AGREE.
            return state;
        }

        let enough_time_has_passed = i64::from(self.nomination().timer_exp_count)
            >= NUM_TIMEOUTS_THRESHOLD_FOR_REPORTING
            || self_already_moved_on;

        match self.nomination().latest_nominations.get(node_id) {
            None => {
                if enough_time_has_passed {
                    state = QuorumInfoNodeState::Missing;
                } else {
                    // Not enough time has passed to start calling this node missing.
                    state = QuorumInfoNodeState::NoInfo;
                }
            }
            Some(other_envelope) => {
                if enough_time_has_passed {
                    if let Some(ref last_envelope) = self.nomination().last_envelope {
                        // Enough time has passed & we have some information on this node.
                        // We'll examine the information to determine the state of this node.
                        if let (
                            proto::SCPStatement_pledges::SCP_ST_NOMINATE {
                                nominate: other_nom,
                            },
                            proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate: my_nom },
                        ) = (
                            &other_envelope.statement.pledges,
                            &last_envelope.statement.pledges,
                        ) {
                            let mut not_equal = false;
                            let is_subset = Self::is_subset(
                                &other_nom.accepted,
                                &my_nom.accepted,
                                &mut not_equal,
                            );

                            if is_subset {
                                if not_equal {
                                    // If there's some values that I've accepted that
                                    // they haven't, they are slow.
                                    state = QuorumInfoNodeState::Delayed;
                                }
                                // else remains AGREE
                            } else {
                                // If they have accepted a value that I haven't,
                                // it's likely that I'll never accept it.
                                state = QuorumInfoNodeState::Disagree;
                            }
                        }
                    }
                }
            }
        }

        state
    }

    pub fn set_state_from_envelope(&mut self, envelope: &proto::SCPEnvelope) {
        if self.nomination().nomination_started {
            panic!("cannot set state after nomination is started");
        }

        self.record_envelope(envelope);

        if let proto::SCPStatement_pledges::SCP_ST_NOMINATE { nominate: nom } =
            &envelope.statement.pledges
        {
            // Add all accepted values from the envelope
            for accepted_value in nom.accepted.iter() {
                self.nomination_mut()
                    .accepted
                    .insert(accepted_value.clone());
            }

            // Add all votes from the envelope
            for vote_value in nom.votes.iter() {
                self.nomination_mut().votes.insert(vote_value.clone());
            }

            // Set this as the last envelope
            self.nomination_mut().last_envelope = Some(envelope.clone());
        } else {
            panic!("bug")
        }
    }

    pub fn process_current_state<F>(&self, f: &mut F, force_self: bool) -> bool
    where
        F: FnMut(&proto::SCPEnvelope) -> bool,
    {
        for (node_id, envelope) in &self.nomination().latest_nominations {
            // only return messages for self if the slot is fully validated
            if force_self
                || node_id != self.slot().local_proto().get_node_id()
                || self.slot().is_fully_validated()
            {
                if !f(envelope) {
                    return false;
                }
            }
        }
        true
    }

    pub fn get_latest_message(&self, node_id: &proto::SCPNodeID) -> Option<proto::SCPEnvelope> {
        self.nomination().latest_nominations.get(node_id).cloned()
    }

    pub fn get_latest_composite_candidate(&self) -> Option<proto::SCPValue> {
        self.nomination().latest_composite_candidate.clone()
    }

    pub fn get_last_message_send(&self) -> Option<proto::SCPEnvelope> {
        self.nomination().last_envelope.clone()
    }
}
