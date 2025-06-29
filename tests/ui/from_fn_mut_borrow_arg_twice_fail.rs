use dynify::{from_fn, Fn};

struct Test;
impl Test {
    fn test<'a, 'b>(&'a self, data: &'b mut [u8]) -> Fn!(&'a Self, &'b mut [u8] => ()) {
        from_fn!(|_: &Self, _: &mut [u8]| {}, self, data)
    }
}

// `from_fn` holds mutable reference exclusively
fn main() {
    let test = Test;
    let mut data = [0u8; 4];

    let init1 = test.test(&mut data);
    let init2 = test.test(&mut data); // fails
    drop((init1, init2));
}
