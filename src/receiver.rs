use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::{Void, VoidPtr};

/// A utility trait used to erase the type of a method receiver.
///
/// This trait is essential to enable a method to return a dyn compatible [`Fn`]
/// constructor.
///
/// # Safety
///
/// The implementor must adhere the documented contracts of each method.
///
/// [`Fn`]: crate::function::Fn
pub unsafe trait Receiver: core::ops::Deref {
    /// The sealed type of this receiver.
    type Sealed;

    /// Erases the type of this method receiver and returns a sealed object.
    ///
    /// The returned value may not be passed to other methods even if they have
    /// the same type.
    fn seal(self) -> Self::Sealed;

    /// Recovers the original type from the sealed object.
    ///
    /// # Safety
    ///
    /// `sealed` must be created from the original receiver of the method to
    /// which it is passed.
    unsafe fn unseal(sealed: Self::Sealed) -> Self;
}

/// The sealed type of `Pin<T>`.
pub struct Pin<T>(T);
unsafe impl<T: Receiver> Receiver for core::pin::Pin<T> {
    type Sealed = Pin<T::Sealed>;
    fn seal(self) -> Self::Sealed {
        unsafe {
            let this = core::pin::Pin::into_inner_unchecked(self);
            Pin(T::seal(this))
        }
    }
    unsafe fn unseal(sealed: Self::Sealed) -> Self {
        core::pin::Pin::new_unchecked(T::unseal(sealed.0))
    }
}

/// The sealed type of `&Self`.
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

/// The sealed type of `&mut Self`.
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
    use alloc::boxed::Box;
    use alloc::rc::Rc;
    use alloc::sync::Arc;

    use super::*;

    struct AllocReceiver {
        data: VoidPtr,
        drop_fn: unsafe fn(VoidPtr),
    }
    impl AllocReceiver {
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

    /// The sealed type of `Box<Self>`.
    pub struct BoxSelf(AllocReceiver);
    unsafe impl<T> Receiver for Box<T> {
        type Sealed = BoxSelf;
        fn seal(self) -> Self::Sealed {
            unsafe fn drop_fn<T>(data: VoidPtr) {
                drop(Box::from_raw(data.cast::<T>().as_ptr()));
            }
            unsafe {
                BoxSelf(AllocReceiver {
                    data: NonNull::new_unchecked(Box::into_raw(self)).cast(),
                    drop_fn: drop_fn::<T>,
                })
            }
        }
        unsafe fn unseal(sealed: Self::Sealed) -> Self {
            let data = sealed.0.into_raw();
            Box::from_raw(data.cast().as_ptr())
        }
    }

    /// The sealed type of `Rc<Self>`.
    pub struct RcSelf(AllocReceiver);
    unsafe impl<T> Receiver for Rc<T> {
        type Sealed = RcSelf;
        fn seal(self) -> Self::Sealed {
            unsafe fn drop_fn<T>(data: VoidPtr) {
                drop(Rc::from_raw(data.cast::<T>().as_ptr()));
            }
            unsafe {
                RcSelf(AllocReceiver {
                    data: NonNull::new_unchecked(Rc::into_raw(self).cast_mut()).cast(),
                    drop_fn: drop_fn::<T>,
                })
            }
        }
        unsafe fn unseal(sealed: Self::Sealed) -> Self {
            let data = sealed.0.into_raw();
            Rc::from_raw(data.cast().as_ptr())
        }
    }

    /// The sealed type of `Arc<Self>`.
    pub struct ArcSelf(AllocReceiver);
    unsafe impl<T> Receiver for Arc<T> {
        type Sealed = ArcSelf;
        fn seal(self) -> Self::Sealed {
            unsafe fn drop_fn<T>(data: VoidPtr) {
                drop(Arc::from_raw(data.cast::<T>().as_ptr()));
            }
            unsafe {
                ArcSelf(AllocReceiver {
                    data: NonNull::new_unchecked(Arc::into_raw(self).cast_mut()).cast(),
                    drop_fn: drop_fn::<T>,
                })
            }
        }
        unsafe fn unseal(sealed: Self::Sealed) -> Self {
            let data = sealed.0.into_raw();
            Arc::from_raw(data.cast().as_ptr())
        }
    }
}
#[cfg(feature = "alloc")]
pub use __alloc::*;

#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use std::rc::Rc;
    use std::sync::Arc;

    use rstest::rstest;

    use super::*;
    use crate::utils::DropCounter;

    struct FakeSelf(i32);

    #[rstest]
    #[case(& FakeSelf(1))]
    #[case(&mut FakeSelf(2))]
    #[case(Box::new(FakeSelf(3)))]
    #[case(Rc::new(FakeSelf(4)))]
    #[case(Arc::new(FakeSelf(5)))]
    #[case(Pin::new(Box::new(FakeSelf(6))))]
    fn unsealed_ptr_matches_original<R>(#[case] orig: R)
    where
        R: Receiver<Target = FakeSelf>,
    {
        let orig_addr = std::ptr::from_ref(&*orig);
        let orig_val = orig.0;
        let sealed = orig.seal();
        let curr = unsafe { R::unseal(sealed) };
        let curr_addr = std::ptr::from_ref(&*curr);
        let curr_val = curr.0;
        assert_eq!(curr_addr, orig_addr);
        assert_eq!(curr_val, orig_val);
    }

    #[rstest]
    #[case(Box::new(DropCounter))]
    #[case(Rc::new(DropCounter))]
    #[case(Arc::new(DropCounter))]
    #[case(Pin::new(Rc::new(DropCounter)))]
    fn sealed_ptr_drop_works(#[case] recv: impl Receiver) {
        assert_eq!(DropCounter::count(), 0);
        drop(recv);
        assert_eq!(DropCounter::count(), 1);
    }
}
