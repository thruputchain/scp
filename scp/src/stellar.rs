use crate::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct ConsensusDriver<C: Consensus, D: Driver> {
    pub consensus: C,
    pub driver: D,
}

impl<C: Consensus, D: Driver> ConsensusDriver<C, D> {
    pub fn new(consensus: C, driver: D) -> ConsensusDriver<C, D> {
        return ConsensusDriver { consensus, driver };
    }
}

#[derive(Debug)]
pub struct Stellar<C: Consensus, D: Driver> {
    cd: ConsensusDriver<C, D>,
    local: LocalState,
    slot: HashMap<SlotIndex, Slot>,
}

impl<'a, C: Consensus, D: Driver> Stellar<C, D> {
    fn slot_proto(&mut self, slot_index: SlotIndex) -> Option<SlotProto<'_, C, D>> {
        self.slot.get_mut(&slot_index).map(|slot| SlotProto {
            cd: &self.cd,
            local: &mut self.local,
            slot,
        })
    }
}

impl<'a, C: Consensus, D: Driver> DriverCb for Stellar<C, D> {
    fn received(&mut self, env: &proto::SCPEnvelope) {
        panic!("XXX")
    }
    fn timer_expired(&mut self, slot_index: SlotIndex, timer: Timer) {
        if let Some(sp) = &mut self.slot_proto(slot_index) {
            sp.timer_expired(timer);
        }
    }
}
