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
