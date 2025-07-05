use std::marker::PhantomPinned;
use std::ops::Deref;
use std::pin::Pin;

use dynify::{from_closure, Buffered, Dynify};

// Wraps Pin::into_inner to prevent rustc from reporting errors from rust-src.
fn unpin<P: Deref>(ptr: Pin<P>) -> P
where
    P::Target: Unpin,
{
    std::pin::Pin::into_inner(ptr)
}

fn main() {
    let mut stack = [0u8; 16];
    let init = from_closure(|slot| slot.write(PhantomPinned));
    let val: Buffered<PhantomPinned> = init.init(&mut stack);

    let pinned = std::pin::pin!(val);
    let _ = unpin(pinned); // fails
}
