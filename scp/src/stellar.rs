use crate::*;
use std::collections::HashMap;

pub struct ConsensusDriver<'a, C: Consensus, D: Driver> {
    pub consensus: &'a C,
    pub driver: &'a D,
}

impl<'a, C: Consensus, D: Driver> ConsensusDriver<'a, C, D> {
    pub fn new(consensus: &'a C, driver: &'a D) -> ConsensusDriver<'a, C, D> {
        return ConsensusDriver { consensus, driver };
    }
}

pub struct Stellar<'a, C: Consensus, D: Driver> {
    cd: ConsensusDriver<'a, C, D>,
    local: Local,
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
