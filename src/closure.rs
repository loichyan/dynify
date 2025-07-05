use core::alloc::Layout;
use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::constructor::{Construct, Opaque, PinConstruct, Slot};

/// The constructor created by [`from_closure`].
#[must_use = "constructor must be initialized"]
pub struct Closure<T, F>(F, PhantomData<T>);
// SAFETY:
// - A typed slot only accepts writes of objects of type `T`, ensuring that the
//   layout of the target object always matches `layout()`.
// - Due to the Higher-Rank Trait Bounds of `F`, references to slots passed to
//   it cannot be moved out, making it impossible to return a slot from an
//   arbitrary address in safe Rust.
// - Once a slot is consumed, it returns an opaque reference to the filled
//   object, which cannot be projected, thus guaranteeing that the layout of its
//   pointee always matches `T`.
unsafe impl<T, U, F> PinConstruct for Closure<T, F>
where
    U: ?Sized,
    F: FnOnce(Slot<T>) -> &mut Opaque<U>,
{
    type Object = U;
    fn layout(&self) -> Layout {
        Layout::new::<T>()
    }
    unsafe fn construct(self, slot: Slot) -> NonNull<Self::Object> {
        let ptr = (self.0)(slot.cast());
        NonNull::from(ptr.as_mut())
    }
}
unsafe impl<T, U, F> Construct for Closure<T, F>
where
    U: ?Sized,
    F: FnOnce(Slot<T>) -> &mut Opaque<U>,
{
}

/// Creates a new closure constructor.
///
/// It accepts a closure `f` that writes an object of type `T` to the provided
/// slot. When the returned instance is ready to be [`construct`]ed, `f` gets
/// invoked, and its return value is then used as the object pointer. The type
/// of the pointee for the returned reference may differ from `T`. In other
/// words, the actual object type of the returned constructor is `U`, which is
/// not necessarily the same as `T`.
///
/// # Example
///
/// ```rust
/// # use dynify::{from_closure, Opaque, PinDynify};
/// # use std::future::Future;
/// # pollster::block_on(async {
/// let fut = async { String::from("(o.O)") };
/// let kmoji = from_closure(|slot| {
///     // The initialized object is selaed in `Opaque`,
///     let init: &mut Opaque<_> = slot.write(fut);
///     // but it doesn't prevent us from coercing it into a trait object.
///     init as &mut Opaque<dyn Future<Output = String>>
/// });
/// assert_eq!(kmoji.pin_boxed().await, "(o.O)");
/// # });
/// ```
///
/// [`construct`]: PinConstruct::construct
#[inline(always)]
pub fn from_closure<T, U, F>(f: F) -> Closure<T, F>
where
    U: ?Sized,
    F: FnOnce(Slot<T>) -> &mut Opaque<U>,
{
    Closure(f, PhantomData)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::{randstr, OpqStrFut};
    use crate::PinDynify;

    #[pollster::test]
    async fn from_closure_works() {
        let inp = randstr(8..64);
        let init = from_closure(|slot| slot.write(async { inp.clone() }) as &mut OpqStrFut);
        assert_eq!(init.pin_boxed().await, inp);
    }
}
