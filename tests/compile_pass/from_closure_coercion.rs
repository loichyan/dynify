use std::any::Any;

fn main() {
    let var = String::from("abc");
    let _ = dynify::from_closure(move |slot| slot.write(var) as &mut dynify::Opaque<dyn Any>);
}
