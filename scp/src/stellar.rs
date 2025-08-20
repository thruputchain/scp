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
    pub cd: ConsensusDriver,
    pub state: &'a mut StellarState,
}

impl<'a> Clone for StellarProto<'static> {
    //XXX: ok?
    fn clone(&self) -> Self {
        unsafe {
            use std::mem;
            StellarProto {
                cd: self.cd.clone(),
                state: mem::transmute_copy(&self.state),
            }
        }
    }
}

impl<'a> StellarProto<'a> {
    //XXX: example returning moved closure that refers to self. ok?
    fn test_callback(&mut self) -> impl FnOnce() {
        move || {
            //XXX: uses the unsafe clone, yes?
            self.slot_proto(0);
        }
    }

    fn driver(&self) -> &dyn Driver {
        self.cd.driver.as_ref()
    }

    fn consensus(&self) -> &dyn Consensus {
        self.cd.consensus.as_ref()
    }

    pub fn slot_proto(&mut self, slot_index: SlotIndex) -> Option<SlotProto<'_>> {
        let self_clone = unsafe { std::mem::transmute_copy(&self) }; //XXX: ok?
        self.state.slot.get_mut(&slot_index).map(|slot| SlotProto {
            proxy: self_clone,
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
