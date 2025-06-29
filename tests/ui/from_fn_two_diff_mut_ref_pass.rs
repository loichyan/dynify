use dynify::{from_fn, Dynify, Fn};

struct Test;
impl Test {
    fn test<'a, 'b>(
        &'a mut self,
        data: &'b mut [u8],
    ) -> Fn!(&'a mut Self, &'b mut [u8] => dyn 'b + Send) {
        from_fn!(|_: &mut Self, d: &'b mut [u8]| d, self, data)
    }
}

// `from_fn` holds mutable reference exclusively
fn main() {
    let mut test = Test;
    let mut data1 = [0u8; 4];
    let mut data2 = [0u8; 4];

    let obj1 = test.test(&mut data1).boxed();
    let obj2 = test.test(&mut data2).boxed();
    drop((obj1, obj2));
}
