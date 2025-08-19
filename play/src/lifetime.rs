use std::cell::RefCell;
use std::rc::Rc;

struct Counter {
    value: i32,
    name: String,
}

impl Counter {
    fn new(name: String) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Counter { value: 0, name }))
    }

    // Immutable methods - work with &self
    fn get_value(&self) -> i32 {
        self.value
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    // Mutable methods - work with &mut self
    fn increment(&mut self) {
        self.value += 1;
    }

    fn set_value(&mut self, value: i32) {
        self.value = value;
    }
}

// Extension trait for convenient method calls
trait CounterExt {
    fn call_increment(&self);
    fn call_get_value(&self) -> i32;
    fn call_get_name(&self) -> String;
    fn call_set_value(&self, value: i32);
}

impl CounterExt for Rc<RefCell<Counter>> {
    fn call_increment(&self) {
        self.borrow_mut().increment();
    }

    fn call_get_value(&self) -> i32 {
        self.borrow().get_value()
    }

    fn call_get_name(&self) -> String {
        self.borrow().get_name().to_string()
    }

    fn call_set_value(&self, value: i32) {
        self.borrow_mut().set_value(value);
    }
}

fn main() {
    let counter = Counter::new("MyCounter".to_string());
    let counter2 = counter.clone();

    // Use through extension trait
    counter.call_increment();
    counter2.call_set_value(10);

    println!("Value: {}", counter.call_get_value());
    println!("Name: {}", counter.call_get_name());
}
