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
pub struct Stellar {
    cd: ConsensusDriver,
    local: LocalState,
    slot: HashMap<SlotIndex, Slot>,
}

impl Stellar {
    fn driver(&self) -> &dyn Driver {
        self.cd.driver.as_ref()
    }

    fn consensus(&self) -> &dyn Consensus {
        self.cd.consensus.as_ref()
    }

    fn slot_proto(&mut self, slot_index: SlotIndex) -> Option<SlotProto<'_>> {
        self.slot.get_mut(&slot_index).map(|slot| SlotProto {
            cd: &self.cd,
            local: &mut self.local,
            slot,
        })
    }
}

impl DriverCb for Stellar {
    fn received(&mut self, env: &proto::SCPEnvelope) {
        panic!("XXX")
    }
    fn timer_expired(&mut self, slot_index: SlotIndex, timer: Timer) {
        if let Some(sp) = &mut self.slot_proto(slot_index) {
            sp.timer_expired(timer);
        }
    }
}
