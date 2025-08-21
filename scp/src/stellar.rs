use crate::*;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct ConsensusDriver {
    pub consensus: Rc<dyn Consensus>,
    pub driver: Rc<dyn Driver>,
}

impl ConsensusDriver {
    pub fn envelope_to_str(&self, env: &proto::SCPEnvelope, full_keys: bool) -> String {
        self.statement_to_str(&env.statement, full_keys)
    }

    pub fn statement_to_str(&self, st: &proto::SCPStatement, full_keys: bool) -> String {
        todo!("XXX")
    }
}

#[derive(Debug)]
pub struct StellarState {
    local: LocalState,
    known_slots: HashMap<SlotIndex, SlotState>,
}

impl StellarState {
    pub fn new(
        node_id: proto::SCPNodeID,
        is_validator: bool,
        qset: proto::SCPQuorumSet,
        get_hash_of: impl Fn(&[u8]) -> proto::SCPHash,
    ) -> StellarState {
        StellarState {
            local: LocalState::new(node_id, is_validator, qset, get_hash_of),
            known_slots: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct StellarProto<'a> {
    pub cd: ConsensusDriver,
    pub state: &'a mut StellarState,
}

impl<'a> StellarProto<'a> {
    pub fn local_proto(&self) -> LocalProto<'_> {
        LocalProto {
            cd: &self.cd,
            local: &self.state.local,
        }
    }

    pub fn local_proto_mut(&mut self) -> LocalProtoMut<'_> {
        LocalProtoMut {
            cd: &self.cd,
            local: &mut self.state.local,
        }
    }

    pub fn slot_proto_or_create(&mut self, slot_index: SlotIndex) -> SlotProto<'_> {
        let is_validator = self.is_validator();
        let known_slots = &mut self.state.known_slots;
        if !known_slots.contains_key(&slot_index) {
            known_slots.insert(slot_index, SlotState::new(is_validator));
        };
        SlotProto {
            cd: &self.cd,
            local_state: &mut self.state.local,
            slot_state: known_slots.get_mut(&slot_index).unwrap(),
        }
    }

    pub fn slot_proto(&mut self, slot_index: SlotIndex) -> Option<SlotProto<'_>> {
        self.state
            .known_slots
            .get_mut(&slot_index)
            .map(|slot| SlotProto {
                cd: &self.cd,
                local_state: &mut self.state.local,
                slot_state: slot,
            })
    }

    pub fn is_validator(&self) -> bool {
        self.local_proto().is_validator()
    }

    pub fn get_highest_known_slot_index(&self) -> SlotIndex {
        self.state.known_slots.keys().max().cloned().unwrap_or(0)
    }

    pub fn nominate(
        &mut self,
        slot_index: SlotIndex,
        value: &proto::SCPValue,
        prev_value: &proto::SCPValue,
    ) -> bool {
        assert!(self.is_validator());
        self.slot_proto_or_create(slot_index)
            .nominate(value, prev_value, false)
    }

    pub fn stop_nomination(&mut self, slot_index: SlotIndex) {
        if let Some(mut slot_proto) = self.slot_proto(slot_index) {
            slot_proto.stop_nomination();
        }
    }

    pub fn update_local_quorum_set(&mut self, qset: &proto::SCPQuorumSet) {
        self.local_proto_mut().update_quorum_set(qset);
    }

    pub fn get_local_quorum_set(&self) -> proto::SCPQuorumSet {
        self.local_proto().get_qset().clone()
    }

    pub fn get_local_node_id(&self) -> proto::SCPNodeID {
        self.local_proto().get_node_id().clone()
    }

    pub fn purge_slots(&mut self, max_slot_index: SlotIndex, slot_to_keep: SlotIndex) {
        let mut slots_to_remove = Vec::new();

        for (&slot_index, _) in self.state.known_slots.iter() {
            if slot_index < max_slot_index && slot_index != slot_to_keep {
                slots_to_remove.push(slot_index);
            }
        }

        for slot_index in slots_to_remove {
            self.state.known_slots.remove(&slot_index);
        }
    }

    pub fn get_json_info(&mut self, limit: usize, full_keys: bool) -> serde_json::Value {
        let mut ret = serde_json::Map::new();

        // Get slot indices in descending order
        let mut slot_indices: Vec<SlotIndex> = self.state.known_slots.keys().cloned().collect();
        slot_indices.sort_by(|a, b| b.cmp(a)); // Sort in descending order

        let mut count = 0;
        for slot_index in slot_indices {
            if count >= limit {
                break;
            }

            if let Some(mut slot_proto) = self.slot_proto(slot_index) {
                ret.insert(slot_index.to_string(), slot_proto.get_json_info(full_keys));
            }
            count += 1;
        }

        serde_json::Value::Object(ret)
    }

    pub fn get_state(
        &mut self,
        node: &proto::SCPNodeID,
        slot_index: SlotIndex,
    ) -> QuorumInfoNodeState {
        const NUM_SLOTS_TO_CHECK_FOR_REPORTING: i32 = 2;

        for k in 0..NUM_SLOTS_TO_CHECK_FOR_REPORTING {
            if slot_index <= k as SlotIndex {
                break;
            }

            let check_slot_index = slot_index - k as SlotIndex;
            if let Some(mut slot) = self.slot_proto(check_slot_index) {
                let self_already_moved_on = k > 0;
                let state = slot.get_state(node, self_already_moved_on);
                if state != QuorumInfoNodeState::NoInfo {
                    return state;
                }
            }
        }

        // We have no messages from this node in N slots.
        // At this point, we'll consider this node MISSING.
        QuorumInfoNodeState::Missing
    }

    pub fn get_json_quorum_info(
        &mut self,
        id: &proto::SCPNodeID,
        summary: bool,
        full_keys: bool,
        index: SlotIndex,
    ) -> serde_json::Value {
        if self.state.known_slots.is_empty() {
            return serde_json::Value::Object(serde_json::Map::new());
        }

        let index = if index == 0 {
            self.get_highest_known_slot_index()
        } else {
            index
        };

        if let Some(mut slot) = self.slot_proto(index) {
            let mut ret = slot.get_json_quorum_info(id, summary, full_keys);
            let qset = self.local_proto().get_qset().clone();

            if summary {
                let mut n_disagree = 0;
                let mut n_missing = 0;
                let mut n_delayed = 0;
                let mut n_agree = 0;

                // Categorize each node's state
                LocalProto::for_all_nodes(&qset, &mut |node| {
                    let state = self.get_state(node, index);
                    match state {
                        QuorumInfoNodeState::Disagree => n_disagree += 1,
                        QuorumInfoNodeState::Delayed => n_delayed += 1,
                        QuorumInfoNodeState::Missing => n_missing += 1,
                        QuorumInfoNodeState::Agree => n_agree += 1,
                        QuorumInfoNodeState::NoInfo => {
                            tracing::error!("get_state should never return NoInfo");
                            return false;
                        }
                    }
                    true
                });

                ret["disagree"] = serde_json::Value::Number(n_disagree.into());
                ret["missing"] = serde_json::Value::Number(n_missing.into());
                ret["delayed"] = serde_json::Value::Number(n_delayed.into());
                ret["agree"] = serde_json::Value::Number(n_agree.into());
            } else {
                let mut disagree = Vec::new();
                let mut missing = Vec::new();
                let mut delayed = Vec::new();
                let mut agree = Vec::new();

                // Categorize each node's state
                LocalProto::for_all_nodes(&qset, &mut |node| {
                    let state = self.get_state(node, index);
                    let key = self.cd.driver.to_str_key(node, full_keys);

                    match state {
                        QuorumInfoNodeState::Disagree => {
                            disagree.push(serde_json::Value::String(key))
                        }
                        QuorumInfoNodeState::Delayed => {
                            delayed.push(serde_json::Value::String(key))
                        }
                        QuorumInfoNodeState::Missing => {
                            missing.push(serde_json::Value::String(key))
                        }
                        QuorumInfoNodeState::Agree => agree.push(serde_json::Value::String(key)),
                        QuorumInfoNodeState::NoInfo => {
                            tracing::error!("get_state should never return NoInfo");
                            return false;
                        }
                    }
                    true
                });

                ret["disagree"] = serde_json::Value::Array(disagree);
                ret["missing"] = serde_json::Value::Array(missing);
                ret["delayed"] = serde_json::Value::Array(delayed);
                ret["agree"] = serde_json::Value::Array(agree);
            }

            ret["ledger"] = serde_json::Value::Number(index.into());
            ret
        } else {
            serde_json::Value::Object(serde_json::Map::new())
        }
    }

    pub fn is_slot_fully_validated(&mut self, slot_index: SlotIndex) -> bool {
        if let Some(slot) = self.slot_proto(slot_index) {
            slot.is_fully_validated()
        } else {
            false
        }
    }

    pub fn got_vblocking(&mut self, slot_index: SlotIndex) -> bool {
        if let Some(slot) = self.slot_proto(slot_index) {
            slot.got_vblocking()
        } else {
            false
        }
    }

    pub fn get_known_slots_count(&self) -> usize {
        self.state.known_slots.len()
    }

    pub fn get_cumulative_statement_count(&mut self) -> usize {
        let mut count = 0;
        for slot_index in self.state.known_slots.keys().cloned().collect::<Vec<_>>() {
            if let Some(slot) = self.slot_proto(slot_index) {
                count += slot.get_statement_count();
            }
        }
        count
    }

    pub fn get_latest_messages_send(&mut self, slot_index: SlotIndex) -> Vec<proto::SCPEnvelope> {
        if let Some(mut slot) = self.slot_proto(slot_index) {
            slot.get_latest_messages_send()
        } else {
            Vec::new()
        }
    }

    pub fn set_state_from_envelope(
        &mut self,
        slot_index: SlotIndex,
        envelope: &proto::SCPEnvelope,
    ) {
        let mut slot = self.slot_proto_or_create(slot_index);
        slot.set_state_from_envelope(envelope);
    }

    pub fn empty(&self) -> bool {
        self.state.known_slots.is_empty()
    }

    pub fn process_current_state<F>(&mut self, slot_index: SlotIndex, f: &mut F, force_self: bool)
    where
        F: FnMut(&proto::SCPEnvelope) -> bool,
    {
        if let Some(mut slot) = self.slot_proto(slot_index) {
            slot.process_current_state(f, force_self)
        }
    }

    pub fn process_slots_ascending_from<F>(&self, starting_slot: SlotIndex, f: &mut F)
    where
        F: FnMut(SlotIndex) -> bool,
    {
        // Get slot indices in ascending order starting from starting_slot
        let mut slot_indices: Vec<SlotIndex> = self
            .state
            .known_slots
            .keys()
            .filter(|&&slot_index| slot_index >= starting_slot)
            .cloned()
            .collect();
        slot_indices.sort();

        for slot_index in slot_indices {
            if !f(slot_index) {
                break;
            }
        }
    }

    pub fn process_slots_descending_from<F>(&self, starting_slot: SlotIndex, f: &mut F)
    where
        F: FnMut(SlotIndex) -> bool,
    {
        // Get slot indices in descending order up to and including starting_slot
        let mut slot_indices: Vec<SlotIndex> = self
            .state
            .known_slots
            .keys()
            .filter(|&&slot_index| slot_index <= starting_slot)
            .cloned()
            .collect();
        slot_indices.sort_by(|a, b| b.cmp(a)); // Sort in descending order

        for slot_index in slot_indices {
            if !f(slot_index) {
                break;
            }
        }
    }
}

impl<'a> DriverCb for StellarProto<'a> {
    fn received_envelope(&mut self, env: &proto::SCPEnvelope) {
        let slot_index = env.statement.slot_index;
        self.slot_proto_or_create(slot_index)
            .process_envelope(env, false);
    }

    fn driver_calling_back(&mut self, cb: DriverCallback) {
        match cb {
            DriverCallback::BallotTimerExpired { slot_index }
            | DriverCallback::NominationTimerExpired {
                slot_index,
                value: _,
                prev_value: _,
            } => {
                if let Some(mut sp) = self.slot_proto(slot_index) {
                    sp.driver_calling_back(cb);
                }
            }
        }
    }
}
