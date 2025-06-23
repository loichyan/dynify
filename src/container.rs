use crate::constructor::{Constructor, PinConstructor, Slot};
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr::NonNull;

/// A one-time container used to construct `dyn` objects.
pub unsafe trait Container<Dyn: ?Sized>: Sized {
    type Ptr: Deref;
    type Err<Args>;

    fn emplace<Args>(self, constructor: Constructor<Dyn, Args>) -> Self::Ptr {
        self.try_emplace(constructor)
            .unwrap_or_else(|_| panic!("failed to initialize"))
    }

    fn try_emplace<Args>(
        self,
        constructor: Constructor<Dyn, Args>,
    ) -> Result<Self::Ptr, Self::Err<Args>>;
}

/// A variant of [`Container`] that requires pinned pointers.
pub unsafe trait PinContainer<Dyn: ?Sized>: Container<Dyn> {
    type PinErr<Args>;

    fn pin_emplace<Args>(self, constructor: Constructor<Dyn, Args>) -> Pin<Self::Ptr> {
        self.try_pin_emplace(constructor)
            .unwrap_or_else(|_| panic!("failed to initialize"))
    }

    fn try_pin_emplace<Args>(
        self,
        constructor: Constructor<Dyn, Args>,
    ) -> Result<Pin<Self::Ptr>, Self::PinErr<Args>>;
}

pub struct Buffered<'a, Dyn: ?Sized>(NonNull<Dyn>, PhantomData<&'a mut [u8]>);
impl<'a, Dyn: ?Sized> Buffered<'a, Dyn> {
    pub unsafe fn from_raw(ptr: NonNull<Dyn>) -> Self {
        Self(ptr, PhantomData)
    }
}
impl<Dyn: ?Sized> Drop for Buffered<'_, Dyn> {
    fn drop(&mut self) {
        unsafe { self.0.drop_in_place() }
    }
}
impl<Dyn: ?Sized> Deref for Buffered<'_, Dyn> {
    type Target = Dyn;
    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}
impl<Dyn: ?Sized> DerefMut for Buffered<'_, Dyn> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}

// Normal buffer
unsafe impl<'a, Dyn: ?Sized, const N: usize> Container<Dyn> for &'a mut [u8; N] {
    type Ptr = Buffered<'a, Dyn>;
    type Err<Args> = Constructor<Dyn, Args>;

    fn try_emplace<Args>(
        self,
        constructor: Constructor<Dyn, Args>,
    ) -> Result<Self::Ptr, Self::Err<Args>> {
        self.as_mut_slice().try_emplace(constructor)
    }
}
unsafe impl<'a, Dyn: ?Sized, const N: usize> Container<Dyn> for &'a mut [MaybeUninit<u8>; N] {
    type Ptr = Buffered<'a, Dyn>;
    type Err<Args> = Constructor<Dyn, Args>;

    fn try_emplace<Args>(
        self,
        constructor: Constructor<Dyn, Args>,
    ) -> Result<Self::Ptr, Self::Err<Args>> {
        self.as_mut_slice().try_emplace(constructor)
    }
}
unsafe impl<'a, Dyn: ?Sized> Container<Dyn> for &'a mut [u8] {
    type Ptr = Buffered<'a, Dyn>;
    type Err<Args> = Constructor<Dyn, Args>;

    fn try_emplace<Args>(
        self,
        constructor: Constructor<Dyn, Args>,
    ) -> Result<Self::Ptr, Self::Err<Args>> {
        let maybe_uninit: &mut [MaybeUninit<u8>] = unsafe { core::mem::transmute(self) };
        maybe_uninit.try_emplace(constructor)
    }
}
unsafe impl<'a, Dyn: ?Sized> Container<Dyn> for &'a mut [MaybeUninit<u8>] {
    type Ptr = Buffered<'a, Dyn>;
    type Err<Args> = Constructor<Dyn, Args>;

    fn try_emplace<Args>(
        self,
        constructor: Constructor<Dyn, Args>,
    ) -> Result<Self::Ptr, Self::Err<Args>> {
        let layout = constructor.layout();
        let capacity = self.len();

        let buf = self.as_mut_ptr().cast::<u8>();
        let align_offset = buf.align_offset(layout.align());
        let total_bytes = align_offset + layout.size();

        if total_bytes > capacity {
            return Err(constructor);
        }
        unsafe {
            let slot = buf.add(align_offset);
            let slot = Slot::new(NonNull::new_unchecked(slot));
            let ptr = constructor.init_unchecked(slot);
            Ok(Buffered::from_raw(ptr))
        }
    }
}

// Pinned buffer
macro_rules! unsafe_impl_pin_buffered {
    (<$lt:lifetime, $Dyn:ident $(,const $N:ident: usize)?> for $ty:ty) => {
        unsafe impl<$lt, $Dyn: ?Sized $(, const $N: usize)*> Container<$Dyn> for Pin<$ty> {
            type Ptr = <$ty as Container<$Dyn>>::Ptr;
            type Err<Args> = <$ty as Container<$Dyn>>::Err<Args>;
            fn emplace<Args>(self, constructor: Constructor<$Dyn, Args>) -> Self::Ptr {
                Pin::into_inner(self).emplace(constructor)
            }
            fn try_emplace<Args>(self, constructor: Constructor<Dyn, Args>) -> Result<Self::Ptr, Self::Err<Args>> {
                Pin::into_inner(self).try_emplace(constructor)
            }
        }
        unsafe impl<$lt, $Dyn: ?Sized $(, const $N: usize)*> PinContainer<$Dyn> for Pin<$ty> {
            type PinErr<Args> = PinConstructor<$Dyn, Args>;
            fn pin_emplace<Args>(self, constructor: Constructor<$Dyn, Args>) -> Pin<Self::Ptr> {
                let ptr = Pin::into_inner(self).emplace(constructor);
                unsafe { Pin::new_unchecked(ptr) }
            }
            fn try_pin_emplace<Args>(self, constructor: Constructor<Dyn, Args>) -> Result<Pin<Self::Ptr>, Self::PinErr<Args>> {
                Pin::into_inner(self)
                    .try_emplace(constructor)
                    .map(|ptr| unsafe { Pin::new_unchecked(ptr) })
                    .map_err(<_>::into)
            }
        }
    };
}
unsafe_impl_pin_buffered!(<'a, Dyn, const N: usize> for &'a mut [u8; N]);
unsafe_impl_pin_buffered!(<'a, Dyn, const N: usize> for &'a mut [MaybeUninit<u8>; N]);
unsafe_impl_pin_buffered!(<'a, Dyn> for &'a mut [u8]);
unsafe_impl_pin_buffered!(<'a, Dyn> for &'a mut [MaybeUninit<u8>]);

#[cfg(feature = "alloc")]
mod __alloc {
    use super::*;
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use core::convert::Infallible;

    // Normal box
    pub struct Boxed;
    unsafe impl<Dyn: ?Sized> Container<Dyn> for Boxed {
        type Ptr = Box<Dyn>;
        type Err<Args> = Infallible;

        fn try_emplace<Args>(
            self,
            constructor: Constructor<Dyn, Args>,
        ) -> Result<Self::Ptr, Self::Err<Args>> {
            let layout = constructor.layout();
            let slot = match layout.size() {
                0 => panic!("zero sized type is not supported"),
                // SAFETY: `layout` is non-zero in size,
                _ => unsafe { NonNull::new(alloc::alloc::alloc(layout)) }
                    .unwrap_or_else(|| alloc::alloc::handle_alloc_error(layout)),
            };
            unsafe {
                let ptr = constructor.init_unchecked(Slot::new(slot));
                Ok(Box::from_raw(ptr.as_ptr()))
            }
        }
    }
    // Pinned box
    unsafe impl<Dyn: ?Sized> PinContainer<Dyn> for Boxed {
        type PinErr<Args> = Infallible;

        fn try_pin_emplace<Args>(
            self,
            constructor: Constructor<Dyn, Args>,
        ) -> Result<Pin<Self::Ptr>, Self::Err<Args>> {
            self.try_emplace(constructor).map(Box::into_pin)
        }
    }

    // Normal vector
    unsafe impl<'a, Dyn: ?Sized> Container<Dyn> for &'a mut Vec<u8> {
        type Ptr = Buffered<'a, Dyn>;
        type Err<Args> = Infallible;

        fn try_emplace<Args>(
            self,
            constructor: Constructor<Dyn, Args>,
        ) -> Result<Self::Ptr, Self::Err<Args>> {
            let maybe_uninit: &mut Vec<MaybeUninit<u8>> = unsafe { core::mem::transmute(self) };
            maybe_uninit.try_emplace(constructor)
        }
    }
    unsafe impl<'a, Dyn: ?Sized> Container<Dyn> for &'a mut Vec<MaybeUninit<u8>> {
        type Ptr = Buffered<'a, Dyn>;
        type Err<Args> = Infallible;

        fn try_emplace<Args>(
            self,
            constructor: Constructor<Dyn, Args>,
        ) -> Result<Self::Ptr, Self::Err<Args>> {
            let layout = constructor.layout();

            loop {
                let capacity = self.capacity();

                let buf = self.as_mut_ptr().cast::<u8>();
                let align_offset = buf.align_offset(layout.align());
                let total_bytes = align_offset + layout.size();

                if total_bytes > capacity {
                    self.reserve(layout.size() + layout.align() - self.len());
                    continue;
                }
                unsafe {
                    let slot = buf.add(align_offset);
                    let slot = Slot::new(NonNull::new_unchecked(slot));
                    let ptr = constructor.init_unchecked(slot);
                    return Ok(Buffered::from_raw(ptr));
                }
            }
        }
    }
    // Pinned vector
    unsafe impl<Dyn: ?Sized> PinContainer<Dyn> for &'_ mut Vec<u8> {
        type PinErr<Args> = Infallible;
        fn try_pin_emplace<Args>(
            self,
            constructor: Constructor<Dyn, Args>,
        ) -> Result<Pin<Self::Ptr>, Self::Err<Args>> {
            self.try_emplace(constructor)
                .map(|ptr| unsafe { Pin::new_unchecked(ptr) })
        }
    }
    unsafe impl<Dyn: ?Sized> PinContainer<Dyn> for &'_ mut Vec<MaybeUninit<u8>> {
        type PinErr<Args> = Infallible;
        fn try_pin_emplace<Args>(
            self,
            constructor: Constructor<Dyn, Args>,
        ) -> Result<Pin<Self::Ptr>, Self::Err<Args>> {
            self.try_emplace(constructor)
                .map(|ptr| unsafe { Pin::new_unchecked(ptr) })
        }
    }
}
#[cfg(feature = "alloc")]
pub use __alloc::*;
