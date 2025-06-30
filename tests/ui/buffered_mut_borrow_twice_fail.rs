use dynify::{from_closure, Buffered, Dynify};

// `Buffered` holds mutable reference exclusively
fn main() {
    let mut stack = [0u8; 16];
    let init1 = from_closure(|slot| slot.write(123));
    let init2 = from_closure(|slot| slot.write(456));

    let val1: Buffered<i32> = init1.init(&mut stack);
    let val2: Buffered<i32> = init2.init(&mut stack); // fails
    drop((val1, val2));
}
