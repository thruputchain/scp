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
    slot: HashMap<SlotIndex, SlotState>,
}

#[derive(Debug)]
pub struct StellarProto<'a> {
    pub cd: ConsensusDriver,
    pub state: &'a mut StellarState,
}

impl<'a> StellarProto<'a> {
    fn driver(&self) -> &dyn Driver {
        self.cd.driver.as_ref()
    }

    fn consensus(&self) -> &dyn Consensus {
        self.cd.consensus.as_ref()
    }

    pub fn slot_proto(&mut self, slot_index: SlotIndex) -> Option<SlotProto<'_>> {
        self.state.slot.get_mut(&slot_index).map(|slot| SlotProto {
            cd: &self.cd,
            local_state: &mut self.state.local,
            slot_state: slot,
        })
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
