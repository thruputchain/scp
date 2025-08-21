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

    pub fn get_highest_known_slot_index(&self) -> SlotIndex {
        self.state.known_slots.keys().max().cloned().unwrap_or(0)
    }
}

impl<'a> DriverCb for StellarProto<'a> {
    fn received_envelope(&mut self, env: &proto::SCPEnvelope) {
        todo!("XXX")
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
