use std::cell::RefCell;
use std::rc::Rc;

struct Slot {
    x: i64,
    ballot: Option<Ballot>,
}

impl Slot {
    fn new() -> Rc<RefCell<Slot>> {
        let slot = Rc::new(RefCell::new(Slot { x: 4, ballot: None }));
        let ballot = Ballot::new(slot.clone());
        slot.borrow_mut().ballot = Some(ballot);
        slot
    }

    fn mut_fn(&mut self) {}
}

struct Ballot {
    slot: Rc<RefCell<Slot>>,
}

impl Ballot {
    fn new(slot: Rc<RefCell<Slot>>) -> Ballot {
        Ballot { slot }
    }

    fn mut_slot(&self) -> std::cell::RefMut<Slot> {
        self.slot.borrow_mut()
    }

    fn other(&self) {
        self.mut_slot().mut_fn();
    }
}
