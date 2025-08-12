use std::sync::{Arc, Mutex, MutexGuard};

struct State {
    x: i64,
}

impl State {
    fn f(&mut self) {
        self.x = 3;
    }
}

struct AState(Arc<Mutex<State>>);

impl AState {
    fn acquire(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap()
    }
    fn f(&self) {
        self.acquire().f()
    }
    fn g(&self) -> AState {
        self.clone()
    }
    fn h(&self) -> impl Fn() {
        move || {
            self.g();
        }
    }
}

impl Clone for AState {
    fn clone(&self) -> Self {
        AState(Arc::clone(&self.0))
    }
}
