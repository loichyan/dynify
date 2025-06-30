use std::marker::PhantomPinned;

use dynify::{from_closure, Buffered, Dynify};

fn main() {
    let mut stack = [0u8; 16];
    let init = from_closure(|slot| slot.write(PhantomPinned));
    let val: Buffered<PhantomPinned> = init.init(&mut stack);

    let pinned = std::pin::pin!(val);
    let _ = std::pin::Pin::into_inner(pinned); // fails
}
