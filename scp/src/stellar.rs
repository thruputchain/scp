use crate::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ConsensusDriver<'a, C: Consensus, D: Driver> {
    pub consensus: &'a C,
    pub driver: &'a D,
}

impl<'a, C: Consensus, D: Driver> ConsensusDriver<'a, C, D> {
    pub fn new(consensus: &'a C, driver: &'a D) -> ConsensusDriver<'a, C, D> {
        return ConsensusDriver { consensus, driver };
    }
}

#[derive(Debug)]
pub struct Stellar<'a, C: Consensus, D: Driver> {
    cd: ConsensusDriver<'a, C, D>,
    local: LocalState,
    slot: HashMap<SlotIndex, Slot>,
}

impl<'a, C: Consensus, D: Driver> Stellar<'a, C, D> {
    fn slot_proto(&mut self, slot_index: SlotIndex) -> Option<SlotProto<'_, C, D>> {
        self.slot.get_mut(&slot_index).map(|slot| SlotProto {
            cd: &self.cd,
            local: &mut self.local,
            slot,
        })
    }
}

impl<'a, C: Consensus, D: Driver> DriverCb for Stellar<'a, C, D> {
    fn received(&mut self, env: &proto::SCPEnvelope) {
        panic!("XXX")
    }
    fn timer_expired(&mut self, slot_index: SlotIndex, timer: Timer) {
        if let Some(sp) = &mut self.slot_proto(slot_index) {
            sp.timer_expired(timer);
        }
    }
}
