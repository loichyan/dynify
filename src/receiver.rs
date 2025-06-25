use crate::{Void, VoidPtr};
use core::marker::PhantomData;
use core::ptr::NonNull;

pub unsafe trait Receiver {
    type Sealed;
    fn seal(self) -> Self::Sealed;
    unsafe fn unseal(sealed: Self::Sealed) -> Self;
}

pub struct RefSelf<'a>(VoidPtr, PhantomData<&'a Void>);
unsafe impl<'a, T> Receiver for &'a T {
    type Sealed = RefSelf<'a>;
    fn seal(self) -> Self::Sealed {
        RefSelf(NonNull::from(self).cast(), PhantomData)
    }
    unsafe fn unseal(sealed: Self::Sealed) -> Self {
        sealed.0.cast().as_ref()
    }
}

pub struct RefMutSelf<'a>(VoidPtr, PhantomData<&'a mut Void>);
unsafe impl<'a, T> Receiver for &'a mut T {
    type Sealed = RefMutSelf<'a>;
    fn seal(self) -> Self::Sealed {
        RefMutSelf(NonNull::from(self).cast(), PhantomData)
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

    pub struct BoxSelf(AllocReceiver);
    unsafe impl<T> Receiver for Box<T> {
        type Sealed = BoxSelf;
        fn seal(self) -> Self::Sealed {
            let data = unsafe { NonNull::new_unchecked(Box::into_raw(self)) };
            BoxSelf(AllocReceiver::new::<Self>(data.cast()))
        }
        unsafe fn unseal(sealed: Self::Sealed) -> Self {
            let data = sealed.0.into_raw();
            Box::from_raw(data.cast().as_ptr())
        }
    }

    pub struct RcSelf(AllocReceiver);
    unsafe impl<T> Receiver for Rc<T> {
        type Sealed = RcSelf;
        fn seal(self) -> Self::Sealed {
            let data = unsafe { NonNull::new_unchecked(Rc::into_raw(self).cast_mut()) };
            RcSelf(AllocReceiver::new::<Self>(data.cast()))
        }
        unsafe fn unseal(sealed: Self::Sealed) -> Self {
            let data = sealed.0.into_raw();
            Rc::from_raw(data.cast().as_ptr())
        }
    }

    pub struct ArcSelf(AllocReceiver);
    unsafe impl<T> Receiver for Arc<T> {
        type Sealed = ArcSelf;
        fn seal(self) -> Self::Sealed {
            let data = unsafe { NonNull::new_unchecked(Arc::into_raw(self).cast_mut()) };
            ArcSelf(AllocReceiver::new::<Self>(data.cast()))
        }
        unsafe fn unseal(sealed: Self::Sealed) -> Self {
            let data = sealed.0.into_raw();
            Arc::from_raw(data.cast().as_ptr())
        }
    }
}
#[cfg(feature = "alloc")]
pub use __alloc::*;
