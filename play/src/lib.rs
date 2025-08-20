pub mod lifetime;

use std::cell::RefCell;
use std::rc::Rc;

trait Driver {
    fn call(&self, cb: Box<dyn FnOnce()>); // without Box?
}

struct XDriver {
    cb: Box<dyn FnOnce()>,
}

struct State {
    sub: Substate,
}

struct Substate {}

#[derive(Clone)]
struct Proto {
    pub driver: Rc<dyn Driver>,
    pub state: Rc<RefCell<State>>,
}

struct Subproto<'a> {
    pub proto: Proto,
    pub sub: &'a mut Substate,
}

impl<'a> Subproto<'a> {
    pub fn g(&mut self) {}
}

impl Proto {
    fn test(&mut self) {
        self.with_sub(|x| x.g());
    }

    fn with_sub<F, R>(&mut self, p: F) -> R
    where
        F: FnOnce(&mut Subproto) -> R,
    {
        let mut sp = Subproto {
            proto: self.clone(),
            sub: &mut self.state.borrow_mut().sub,
        };
        p(&mut sp)
    }

    fn call_driver(&mut self) {
        let mut s = self.clone();
        self.driver.call(Box::new(move || {
            s.other();
        }))
    }
    fn other(&mut self) {}
}
