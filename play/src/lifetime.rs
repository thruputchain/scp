struct X;

struct A<'a>(pub &'a mut X);

impl<'a> A<'a> {
    fn to_b(&mut self) -> B<'_, 'a> {
        B(self)
    }
}

struct B<'a, 'b>(pub &'b mut A<'a>);

impl<'a, 'b> B<'a, 'b> {
    fn to_a_ref(&mut self) -> &mut A<'a> {
        self.0
    }

    fn to_a(&mut self) -> A<'_> {
        A(self.0.0)
    }
}
