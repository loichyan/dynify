use dynify::{from_fn, Fn};

struct Test;
impl Test {
    fn test(&mut self) -> Fn!(&mut Self => ()) {
        from_fn!(|_: &mut Self| {}, self)
    }
}

// `from_fn` holds mutable reference exclusively
fn main() {
    let mut test = Test;

    let init1 = test.test();
    let init2 = test.test(); // fails
    drop((init1, init2));
}
