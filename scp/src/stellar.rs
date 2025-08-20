use crate::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct ConsensusDriver {
    pub consensus: Rc<dyn Consensus>,
    pub driver: Rc<dyn Driver>,
}

#[derive(Debug)]
pub struct StellarState {
    local: LocalState,
    slot: HashMap<SlotIndex, Slot>,
}

#[derive(Debug)]
pub struct StellarProto<'a> {
    cd: ConsensusDriver,
    state: &'a mut StellarState,
}

impl<'a> StellarProto<'a> {
    fn driver(&self) -> &dyn Driver {
        self.cd.driver.as_ref()
    }

    fn consensus(&self) -> &dyn Consensus {
        self.cd.consensus.as_ref()
    }

    fn slot_proto(&mut self, slot_index: SlotIndex) -> Option<SlotProto<'_>> {
        self.state.slot.get_mut(&slot_index).map(|slot| SlotProto {
            cd: &self.cd,
            local: &mut self.state.local,
            slot,
        })
    }
}

impl<'a> DriverCb for StellarProto<'a> {
    fn received(&mut self, env: &proto::SCPEnvelope) {
        panic!("XXX")
    }
    fn timer_expired(&mut self, slot_index: SlotIndex, timer: Timer) {
        if let Some(sp) = &mut self.slot_proto(slot_index) {
            sp.timer_expired(timer);
        }
    }
}
