use core::alloc::Layout;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

use crate::constructor::{Construct, PinConstruct, Slot};

/// The constructor created by [`from_closure`].
pub struct Closure<T, F>(F, PhantomData<T>);
unsafe impl<T, U, F> PinConstruct for Closure<T, F>
where
    U: ?Sized,
    F: FnOnce(&mut MaybeUninit<T>) -> &mut U,
{
    type Object = U;
    fn layout(&self) -> Layout {
        Layout::new::<T>()
    }
    unsafe fn construct(self, slot: Slot) -> NonNull<Self::Object> {
        let mut uninit = slot.write(MaybeUninit::<T>::uninit());
        let ptr = (self.0)(uninit.as_mut());
        assert_eq!(ptr as *const U as *const (), uninit.as_ptr() as *const ());
        assert_eq!(Layout::for_value(ptr), Layout::new::<T>());
        NonNull::from(ptr)
    }
}
unsafe impl<T, U, F> Construct for Closure<T, F>
where
    U: ?Sized,
    F: FnOnce(&mut MaybeUninit<T>) -> &mut U,
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
/// # use dynify::{Construct, from_closure};
/// # use std::future::Future;
/// # pollster::block_on(async {
/// let fut = async { String::from("(o.O)") };
/// let kmoji = from_closure(|slot| slot.write(fut) as &mut dyn Future<Output = String>);
/// assert_eq!(kmoji.dynify().pin_boxed().await, "(o.O)");
/// # });
/// ```
///
/// # Panic
///
/// This function itself does not panic, but if `f` returns a reference that
/// violates the construction contract, that is, the reference has a different
/// address or pointee layout than the provided slot, it will trigger a panic
/// during the [`construct`] method of the returned instance.
///
/// [`construct`]: PinConstruct::construct
pub const fn from_closure<T, U, F>(f: F) -> Closure<T, F>
where
    U: ?Sized,
    F: FnOnce(&mut MaybeUninit<T>) -> &mut U,
{
    Closure(f, PhantomData)
}
