use crate::{Void, VoidPtr};
use core::marker::PhantomData;
use core::ptr::NonNull;

pub trait Receiver {
    type Sealed;
    fn seal(self) -> Self::Sealed;
    unsafe fn unseal(sealed: Self::Sealed) -> Self;
}

pub struct ReceiverRef<'a>(VoidPtr, PhantomData<&'a Void>);
impl<'a, T> Receiver for &'a T {
    type Sealed = ReceiverRef<'a>;
    fn seal(self) -> Self::Sealed {
        ReceiverRef(NonNull::from(self).cast(), PhantomData)
    }
    unsafe fn unseal(sealed: Self::Sealed) -> Self {
        sealed.0.cast().as_ref()
    }
}

pub struct ReceiverMut<'a>(VoidPtr, PhantomData<&'a mut Void>);
impl<'a, T> Receiver for &'a mut T {
    type Sealed = ReceiverMut<'a>;
    fn seal(self) -> Self::Sealed {
        ReceiverMut(NonNull::from(self).cast(), PhantomData)
    }
    unsafe fn unseal(sealed: Self::Sealed) -> Self {
        sealed.0.cast().as_mut()
    }
}

#[cfg(feature = "alloc")]
mod __alloc {
    use super::*;
    use alloc::boxed::Box;
    use alloc::rc::Rc;
    use alloc::sync::Arc;

    struct AllocReceiver {
        data: VoidPtr,
        drop_fn: unsafe fn(VoidPtr),
    }
    impl AllocReceiver {
        fn new<T>(data: VoidPtr) -> Self {
            unsafe fn drop_fn<T>(data: VoidPtr) {
                data.cast::<T>().drop_in_place();
            }
            Self {
                data,
                drop_fn: drop_fn::<T>,
            }
        }
        fn into_raw(self) -> VoidPtr {
            let data = self.data;
            core::mem::forget(self);
            data
        }
    }
    impl Drop for AllocReceiver {
        fn drop(&mut self) {
            unsafe { (self.drop_fn)(self.data) }
        }
    }

    pub struct ReceiverBox(AllocReceiver);
    impl<T> Receiver for Box<T> {
        type Sealed = ReceiverBox;
        fn seal(self) -> Self::Sealed {
            let data = unsafe { NonNull::new_unchecked(Box::into_raw(self)) };
            ReceiverBox(AllocReceiver::new::<Self>(data.cast()))
        }
        unsafe fn unseal(sealed: Self::Sealed) -> Self {
            let data = sealed.0.into_raw();
            Box::from_raw(data.cast().as_ptr())
        }
    }

    pub struct ReceiverRc(AllocReceiver);
    impl<T> Receiver for Rc<T> {
        type Sealed = ReceiverRc;
        fn seal(self) -> Self::Sealed {
            let data = unsafe { NonNull::new_unchecked(Rc::into_raw(self).cast_mut()) };
            ReceiverRc(AllocReceiver::new::<Self>(data.cast()))
        }
        unsafe fn unseal(sealed: Self::Sealed) -> Self {
            let data = sealed.0.into_raw();
            Rc::from_raw(data.cast().as_ptr())
        }
    }

    pub struct ReceiverArc(AllocReceiver);
    impl<T> Receiver for Arc<T> {
        type Sealed = ReceiverArc;
        fn seal(self) -> Self::Sealed {
            let data = unsafe { NonNull::new_unchecked(Arc::into_raw(self).cast_mut()) };
            ReceiverArc(AllocReceiver::new::<Self>(data.cast()))
        }
        unsafe fn unseal(sealed: Self::Sealed) -> Self {
            let data = sealed.0.into_raw();
            Arc::from_raw(data.cast().as_ptr())
        }
    }
}
#[cfg(feature = "alloc")]
pub use __alloc::*;
