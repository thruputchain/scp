struct A {
    b: B,
}

impl A {
    fn x(&mut self) {
        // self.b.f(&self.b)
    }
}

struct B {}

impl B {
    fn f(&mut self, self2: &B) {}
}
