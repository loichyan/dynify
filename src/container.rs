use crate::constructor::{Constructor, Initializer, Slot};
use core::alloc::Layout;
use core::fmt;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr::NonNull;

/// A one-time container used for in-place constructions.
pub unsafe trait Container<T: ?Sized>: Sized {
    type Ptr: Deref;
    type Err;

    /// Consumes this container and initializes the supplied constructor in it.
    ///
    /// If `self` cannot fit the layout of the object to be constructed, it does
    /// nothing and returns an error. Otherwise, it returns a pointer to the
    /// constructed object.
    fn emplace<C>(self, init: &mut Initializer<C>) -> Result<Self::Ptr, Self::Err>
    where
        C: Constructor<Object = T>;
}

/// A pinned version of [`Container`] used for constructions that require pinned
/// memory blocks.
pub unsafe trait PinContainer<T: ?Sized>: Container<T> {
    fn pin_emplace<C>(self, init: &mut Initializer<C>) -> Result<Pin<Self::Ptr>, Self::Err>
    where
        C: Constructor<Object = T>;
}

pub struct Buffered<'a, T: ?Sized>(NonNull<T>, PhantomData<&'a mut [u8]>);
impl<'a, T: ?Sized> Buffered<'a, T> {
    pub unsafe fn from_raw(ptr: NonNull<T>) -> Self {
        Self(ptr, PhantomData)
    }
}
impl<T: ?Sized> Drop for Buffered<'_, T> {
    fn drop(&mut self) {
        unsafe { self.0.drop_in_place() }
    }
}
impl<T: ?Sized> Deref for Buffered<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}
impl<T: ?Sized> DerefMut for Buffered<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}

#[derive(Debug)]
pub struct OutOfCapacity;
impl fmt::Display for OutOfCapacity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("out of capacity")
    }
}

// Normal buffer
unsafe impl<'a, T: ?Sized, const N: usize> Container<T> for &'a mut [u8; N] {
    type Ptr = Buffered<'a, T>;
    type Err = OutOfCapacity;

    fn emplace<C>(self, init: &mut Initializer<C>) -> Result<Self::Ptr, Self::Err>
    where
        C: Constructor<Object = T>,
    {
        self.as_mut_slice().emplace(init)
    }
}
unsafe impl<'a, T: ?Sized, const N: usize> Container<T> for &'a mut [MaybeUninit<u8>; N] {
    type Ptr = Buffered<'a, T>;
    type Err = OutOfCapacity;

    fn emplace<C>(self, init: &mut Initializer<C>) -> Result<Self::Ptr, Self::Err>
    where
        C: Constructor<Object = T>,
    {
        self.as_mut_slice().emplace(init)
    }
}
unsafe impl<'a, T: ?Sized> Container<T> for &'a mut [u8] {
    type Ptr = Buffered<'a, T>;
    type Err = OutOfCapacity;

    fn emplace<C>(self, init: &mut Initializer<C>) -> Result<Self::Ptr, Self::Err>
    where
        C: Constructor<Object = T>,
    {
        let maybe_uninit: &mut [MaybeUninit<u8>] = unsafe { core::mem::transmute(self) };
        maybe_uninit.emplace(init)
    }
}
unsafe impl<'a, T: ?Sized> Container<T> for &'a mut [MaybeUninit<u8>] {
    type Ptr = Buffered<'a, T>;
    type Err = OutOfCapacity;

    fn emplace<C>(self, init: &mut Initializer<C>) -> Result<Self::Ptr, Self::Err>
    where
        C: Constructor<Object = T>,
    {
        unsafe { buf_emplace(self, init.layout(), &mut |slot| init.init_unchecked(slot)) }
    }
}
unsafe fn buf_emplace<'a, T: ?Sized>(
    buf: &'a mut [MaybeUninit<u8>],
    layout: Layout,
    init: &mut dyn FnMut(Slot) -> NonNull<T>,
) -> Result<Buffered<'a, T>, OutOfCapacity> {
    let start = buf.as_mut_ptr().cast::<u8>();
    let align_offset = start.align_offset(layout.align());
    let total_bytes = align_offset + layout.size();

    if total_bytes > buf.len() {
        return Err(OutOfCapacity);
    }
    unsafe {
        let slot = NonNull::new_unchecked(start.add(align_offset));
        let ptr = init(Slot::new(slot));
        Ok(Buffered::from_raw(ptr))
    }
}

// Pinned buffer
macro_rules! unsafe_impl_pin_buffered {
    (<$lt:lifetime, $T:ident $(,const $N:ident: usize)?> for $ty:ty) => {
        unsafe impl<$lt, $T: ?Sized $(, const $N: usize)*> Container<$T> for Pin<$ty> {
            type Ptr = Buffered<$lt, $T>;
            type Err = OutOfCapacity;
            fn emplace<C>(self, init: & mut Initializer<C>) -> Result<Self::Ptr, Self::Err>
            where
                C: Constructor<Object = $T>,
            {
                Pin::into_inner(self).emplace(init)
            }
        }
        unsafe impl<$lt, $T: ?Sized $(, const $N: usize)*> PinContainer<$T> for Pin<$ty> {
            fn pin_emplace<C>(self, init: & mut Initializer<C>) -> Result<Pin<Self::Ptr>, Self::Err>
            where
                C: Constructor<Object = $T>,
            {
                Pin::into_inner(self)
                    .emplace(init)
                    .map(|ptr| unsafe { Pin::new_unchecked(ptr) })
            }
        }
    };
}
unsafe_impl_pin_buffered!(<'a, T, const N: usize> for &'a mut [u8; N]);
unsafe_impl_pin_buffered!(<'a, T, const N: usize> for &'a mut [MaybeUninit<u8>; N]);
unsafe_impl_pin_buffered!(<'a, T> for &'a mut [u8]);
unsafe_impl_pin_buffered!(<'a, T> for &'a mut [MaybeUninit<u8>]);

#[cfg(feature = "alloc")]
mod __alloc {
    use super::*;
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use core::convert::Infallible;

    // Normal box
    pub struct Boxed;
    unsafe impl<T: ?Sized> Container<T> for Boxed {
        type Ptr = Box<T>;
        type Err = Infallible;

        fn emplace<C>(self, init: &mut Initializer<C>) -> Result<Self::Ptr, Self::Err>
        where
            C: Constructor<Object = T>,
        {
            Ok(unsafe { box_emlace(init.layout(), &mut |slot| init.init_unchecked(slot)) })
        }
    }
    unsafe fn box_emlace<T: ?Sized>(
        layout: Layout,
        init: &mut dyn FnMut(Slot) -> NonNull<T>,
    ) -> Box<T> {
        let slot = match layout.size() {
            // TODO: support ZST
            0 => panic!("zero sized type is not supported"),
            // SAFETY: `layout` is non-zero in size,
            _ => unsafe { NonNull::new(alloc::alloc::alloc(layout)) }
                .unwrap_or_else(|| alloc::alloc::handle_alloc_error(layout)),
        };
        unsafe {
            let ptr = init(Slot::new(slot));
            Box::from_raw(ptr.as_ptr())
        }
    }

    // Pinned box
    unsafe impl<T: ?Sized> PinContainer<T> for Boxed {
        fn pin_emplace<C>(self, init: &mut Initializer<C>) -> Result<Pin<Self::Ptr>, Self::Err>
        where
            C: Constructor<Object = T>,
        {
            self.emplace(init).map(Box::into_pin)
        }
    }

    // Normal vector
    unsafe impl<'a, T: ?Sized> Container<T> for &'a mut Vec<u8> {
        type Ptr = Buffered<'a, T>;
        type Err = Infallible;

        fn emplace<C>(self, init: &mut Initializer<C>) -> Result<Self::Ptr, Self::Err>
        where
            C: Constructor<Object = T>,
        {
            let maybe_uninit: &mut Vec<MaybeUninit<u8>> = unsafe { core::mem::transmute(self) };
            maybe_uninit.emplace(init)
        }
    }
    unsafe impl<'a, T: ?Sized> Container<T> for &'a mut Vec<MaybeUninit<u8>> {
        type Ptr = Buffered<'a, T>;
        type Err = Infallible;

        fn emplace<C>(self, init: &mut Initializer<C>) -> Result<Self::Ptr, Self::Err>
        where
            C: Constructor<Object = T>,
        {
            Ok(unsafe { vec_emplace(self, init.layout(), &mut |slot| init.init_unchecked(slot)) })
        }
    }
    unsafe fn vec_emplace<'a, T: ?Sized>(
        vec: &'a mut Vec<MaybeUninit<u8>>,
        layout: Layout,
        init: &mut dyn FnMut(Slot) -> NonNull<T>,
    ) -> Buffered<'a, T> {
        loop {
            let buf = vec.as_mut_ptr().cast::<u8>();
            let align_offset = buf.align_offset(layout.align());
            let total_bytes = align_offset + layout.size();

            if total_bytes > vec.capacity() {
                vec.reserve(layout.size() + layout.align() - vec.len());
                continue;
            }
            unsafe {
                let slot = NonNull::new_unchecked(buf.add(align_offset));
                let ptr = init(Slot::new(slot));
                return Buffered::from_raw(ptr);
            }
        }
    }

    // Pinned vector
    unsafe impl<T: ?Sized> PinContainer<T> for &'_ mut Vec<u8> {
        fn pin_emplace<C>(self, init: &mut Initializer<C>) -> Result<Pin<Self::Ptr>, Self::Err>
        where
            C: Constructor<Object = T>,
        {
            self.emplace(init)
                .map(|ptr| unsafe { Pin::new_unchecked(ptr) })
        }
    }
    unsafe impl<T: ?Sized> PinContainer<T> for &'_ mut Vec<MaybeUninit<u8>> {
        fn pin_emplace<C>(self, init: &mut Initializer<C>) -> Result<Pin<Self::Ptr>, Self::Err>
        where
            C: Constructor<Object = T>,
        {
            self.emplace(init)
                .map(|ptr| unsafe { Pin::new_unchecked(ptr) })
        }
    }
}
#[cfg(feature = "alloc")]
pub use __alloc::*;
