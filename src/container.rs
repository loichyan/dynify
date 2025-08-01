use core::alloc::Layout;
use core::any::Any;
use core::fmt;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr::NonNull;

use crate::constructor::{Construct, PinConstruct, Slot};

/// A one-time container used for in-place constructions.
///
/// A type that implements [`Emplace`] is called a *container*. Each container
/// holds a unique memory block for object constructions. This memory block may
/// have a fixed capacity or grow dynamically. A fixed-size container may reject
/// construction if it lacks sufficient free space to put the target object. In
/// this case, the caller is responsible for preserving the provided
/// constructor, which can be done by wrapping the constructor in [`Option`].
///
/// # Safety
///
/// For the implementor,
///
/// - It must adhere the documented contracts of each method.
/// - If [`emplace`] succeeds, the provided constructor must be consumed through
///   [`construct`]. Conversely, `constructor` must remain untouched if
///   [`emplace`] returns an error. Failing to follow either case results in
///   *undefined behavior*.
///
/// [`construct`]: PinConstruct::construct
/// [`emplace`]: Self::emplace
pub unsafe trait Emplace<T: ?Sized>: Sized {
    type Ptr: core::ops::Deref<Target = T>;
    type Err;

    /// Consumes this container and initializes the supplied constructor in it.
    ///
    /// If `self` cannot fit the layout of the target object, it does nothing
    /// and returns an error. Otherwise, it consumes `constructor` and returns the
    /// pointer to the constructed object.
    fn emplace<C>(self, constructor: C) -> Result<Self::Ptr, Self::Err>
    where
        C: Construct<Object = T>;
}

/// A variant of [`Emplace`] used for pinned constructions.
///
/// A *pinned container* holds a memory block with a stable address, which
/// enables it to store objects that cannot be moved once constructed.
///
/// # Safety
///
/// See the safety notes of [`Emplace`]. Additionally, the implementor must
/// uphold the pinning requirements for the constructed objects.
pub unsafe trait PinEmplace<T: ?Sized>: Emplace<T> {
    /// Initializes the supplied constructor in a pinned memory block.
    ///
    /// It returns a pinned pointer to the constructed object if successful. For
    /// more information, see [`emplace`].
    ///
    /// [`emplace`]: Emplace::emplace
    fn pin_emplace<C>(self, constructor: C) -> Result<Pin<Self::Ptr>, Self::Err>
    where
        C: PinConstruct<Object = T>,
    {
        struct UncheckedPinConstructor<C>(C);
        unsafe impl<C: PinConstruct> PinConstruct for UncheckedPinConstructor<C> {
            type Object = C::Object;
            fn layout(&self) -> Layout {
                self.0.layout()
            }
            unsafe fn construct(self, slot: Slot) -> NonNull<Self::Object> {
                self.0.construct(slot)
            }
        }
        unsafe impl<C: PinConstruct> Construct for UncheckedPinConstructor<C> {}
        self.emplace(UncheckedPinConstructor(constructor))
            .map(|p| unsafe { Pin::new_unchecked(p) })
    }
}

/// A pointer to objects stored in buffers.
///
/// Containers such as `&mut [u8]` or `&mut Vec<u8>` yield this pointer type.
/// Note that, unlike most pointer types, it implements `Unpin` only if `T` is
/// `Unpin`. While this may seem counterintuitive, it simplifies obtaining a
/// pinned reference to `T` in safe Rust, as illustrated below:
///
/// ```rust
/// # use dynify::{from_fn, Buffered, Dynify, Fn};
/// # use std::future::Future;
/// # use std::mem::MaybeUninit;
/// # use std::pin::Pin;
/// # pollster::block_on(async {
/// fn async_hello() -> Fn!(=> dyn Future<Output = String>) {
///     from_fn!(|| async { String::from("Hello!") })
/// }
///
/// let mut stack = MaybeUninit::<[u8; 16]>::uninit();
/// let fut: Buffered<dyn Future<Output = String>> = async_hello().init(&mut stack);
/// // Pin it on the stack just as it has the inner type `T = dyn Future`.
/// let fut: Pin<&mut Buffered<_>> = std::pin::pin!(fut);
/// // Then project it to obtain a pinned reference to `T`.
/// let fut: Pin<&mut dyn Future<Output = String>> = fut.project();
/// assert_eq!(fut.await, "Hello!");
/// # });
/// ```
///
/// **Tips**: `Buffered<T: Future>` implements `Future`, so you can simply write
/// `async_hello().init(&mut stack).await` in practice.
pub struct Buffered<'a, T: ?Sized>(NonNull<T>, PhantomData<&'a mut [u8]>);
impl<'a, T: ?Sized> Buffered<'a, T> {
    /// Constructs a new instance with the provided pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to `T` and exclusive for the returned
    /// instance.
    pub unsafe fn from_raw(ptr: NonNull<T>) -> Self {
        Self(ptr, PhantomData)
    }

    /// Consumes this instance, returning a raw pointer.
    pub fn into_raw(self) -> NonNull<T> {
        let ptr = self.0;
        core::mem::forget(self);
        ptr
    }

    /// Returns a pinned mutable reference to the inner value.
    pub fn project(self: Pin<&mut Self>) -> Pin<&mut T> {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            Pin::new_unchecked(this)
        }
    }

    /// Returns a pinned immutable reference to the inner value.
    pub fn project_ref(self: Pin<&Self>) -> Pin<&T> {
        unsafe {
            let this = Pin::into_inner_unchecked(self);
            Pin::new_unchecked(this)
        }
    }
}
impl<'a, T> Buffered<'a, T> {
    /// Consumes this instance, returning the inner value.
    pub fn into_inner(self) -> T {
        unsafe { self.into_raw().read() }
    }
}
impl<'a> Buffered<'a, dyn Any> {
    /// Attempts to downcast the pointer to a concrete type.
    pub fn downcast<T: Any>(self) -> Result<Buffered<'a, T>, Self> {
        if self.is::<T>() {
            unsafe { Ok(self.downcast_unchecked()) }
        } else {
            Err(self)
        }
    }

    /// Downcasts the box to a concrete type.
    ///
    /// For a safe alternative see [`downcast`](Self::downcast).
    ///
    /// # Safety
    ///
    /// The contained value must be of type `T`.
    pub unsafe fn downcast_unchecked<T: Any>(self) -> Buffered<'a, T> {
        Buffered::from_raw(self.into_raw().cast())
    }
}
impl<'a> Buffered<'a, dyn Any + Send> {
    /// Attempts to downcast the pointer to a concrete type.
    pub fn downcast<T: Any>(self) -> Result<Buffered<'a, T>, Self> {
        if self.is::<T>() {
            unsafe { Ok(self.downcast_unchecked()) }
        } else {
            Err(self)
        }
    }

    /// Downcasts the box to a concrete type.
    ///
    /// For a safe alternative see [`downcast`](Self::downcast).
    ///
    /// # Safety
    ///
    /// The contained value must be of type `T`.
    pub unsafe fn downcast_unchecked<T: Any>(self) -> Buffered<'a, T> {
        Buffered::from_raw(self.into_raw().cast())
    }
}
impl<'a> Buffered<'a, dyn Any + Send + Sync> {
    /// Attempts to downcast the pointer to a concrete type.
    pub fn downcast<T: Any>(self) -> Result<Buffered<'a, T>, Self> {
        if self.is::<T>() {
            unsafe { Ok(self.downcast_unchecked()) }
        } else {
            Err(self)
        }
    }

    /// Downcasts the box to a concrete type.
    ///
    /// For a safe alternative see [`downcast`](Self::downcast).
    ///
    /// # Safety
    ///
    /// The contained value must be of type `T`.
    pub unsafe fn downcast_unchecked<T: Any>(self) -> Buffered<'a, T> {
        Buffered::from_raw(self.into_raw().cast())
    }
}

// Pretend `Buffered` owns the value of `T` rather than just a pointer to it.
// This, along with the `Buffered::project*` APIs, makes it easy to obtain a
// pinned reference to `T` in safe Rust. But the downside is that this prevents
// `Buffered` from being `Unpin` if `T` is not `Unpin`, which is unexpected for
// a pointer type. Nevertheless, in most cases, pinning a pointer is not
// particularly useful.
//
// Besides, we cannot provide `Buffered::into_pin` even if the container has a
// `'static` lifetime. This is because containers do not guarantee that the
// memory region allocated to us will not be overwritten if `Buffered` is leaked
// through `std::mem::forget`. This violation undermines the drop guarantee
// required by `Pin`. For more information, see <https://github.com/fitzgen/bumpalo/issues/186>.
impl<T: ?Sized + Unpin> Unpin for Buffered<'_, T> {}
impl<T: ?Sized> Drop for Buffered<'_, T> {
    fn drop(&mut self) {
        if core::mem::needs_drop::<T>() {
            unsafe { self.0.drop_in_place() }
        }
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

impl<T: ?Sized + fmt::Debug> fmt::Debug for Buffered<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}

impl<T> core::future::Future for Buffered<'_, T>
where
    T: ?Sized + core::future::Future,
{
    type Output = T::Output;
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        self.project().poll(cx)
    }
}

/// An error thrown by buffers with fixed capacity.
#[derive(Debug)]
pub struct OutOfCapacity;
impl fmt::Display for OutOfCapacity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("out of capacity")
    }
}

unsafe impl<'a, T: ?Sized, const N: usize> Emplace<T> for &'a mut MaybeUninit<[u8; N]> {
    type Ptr = Buffered<'a, T>;
    type Err = OutOfCapacity;

    fn emplace<C>(self, constructor: C) -> Result<Self::Ptr, Self::Err>
    where
        C: Construct<Object = T>,
    {
        let uninit_slice: &mut [MaybeUninit<u8>; N] = unsafe { core::mem::transmute(self) };
        uninit_slice.emplace(constructor)
    }
}
unsafe impl<'a, T: ?Sized, const N: usize> Emplace<T> for &'a mut [MaybeUninit<u8>; N] {
    type Ptr = Buffered<'a, T>;
    type Err = OutOfCapacity;

    fn emplace<C>(self, constructor: C) -> Result<Self::Ptr, Self::Err>
    where
        C: Construct<Object = T>,
    {
        self.as_mut_slice().emplace(constructor)
    }
}
unsafe impl<'a, T: ?Sized> Emplace<T> for &'a mut [MaybeUninit<u8>] {
    type Ptr = Buffered<'a, T>;
    type Err = OutOfCapacity;

    fn emplace<C>(self, constructor: C) -> Result<Self::Ptr, Self::Err>
    where
        C: Construct<Object = T>,
    {
        unsafe {
            let layout = constructor.layout();
            let slot = buf_emplace(self, layout)?;
            let ptr = slot.as_ptr();

            let init = constructor.construct(slot);
            validate_slot(ptr, layout, init);
            Ok(Buffered::from_raw(init))
        }
    }
}
unsafe fn buf_emplace(
    buf: &mut [MaybeUninit<u8>],
    layout: Layout,
) -> Result<Slot<'_>, OutOfCapacity> {
    if layout.size() == 0 {
        return Ok(dangling_slot(layout));
    }

    let start = buf.as_mut_ptr();
    let align_offset = start.align_offset(layout.align());
    let total_bytes = align_offset + layout.size();

    if total_bytes > buf.len() {
        return Err(OutOfCapacity);
    }
    let ptr = start.add(align_offset).cast::<u8>();
    Ok(Slot::new_unchecked(NonNull::new_unchecked(ptr)))
}

#[cfg(feature = "alloc")]
mod __alloc {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use core::convert::Infallible;

    use super::*;

    /// A unit type to perform constructions in [`Box`].
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    #[derive(Debug)]
    pub struct Boxed;

    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    unsafe impl<T: ?Sized> Emplace<T> for Boxed {
        type Ptr = Box<T>;
        type Err = Infallible;

        fn emplace<C>(self, constructor: C) -> Result<Self::Ptr, Self::Err>
        where
            C: Construct<Object = T>,
        {
            unsafe {
                let layout = constructor.layout();
                let slot = box_emlace(layout);
                let ptr = slot.as_ptr();

                // Recycle the allocated memory to prevent memory leaks if
                // `construct()` panics.
                let clean_on_panic = crate::utils::defer(|| {
                    if layout.size() != 0 {
                        alloc::alloc::dealloc(ptr.as_ptr(), layout)
                    }
                });
                let init = constructor.construct(slot);
                validate_slot(ptr, layout, init);

                core::mem::forget(clean_on_panic);
                Ok(Box::from_raw(init.as_ptr()))
            }
        }
    }
    // Pinned box
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    unsafe impl<T: ?Sized> PinEmplace<T> for Boxed {}
    unsafe fn box_emlace(layout: Layout) -> Slot<'static> {
        if layout.size() == 0 {
            return dangling_slot(layout);
        }
        // SAFETY: `layout` is non-zero in size,
        let ptr = NonNull::new(alloc::alloc::alloc(layout))
            .unwrap_or_else(|| alloc::alloc::handle_alloc_error(layout));
        Slot::new_unchecked(ptr)
    }

    // TODO: pinned vector?
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    unsafe impl<'a, T: ?Sized> Emplace<T> for &'a mut Vec<MaybeUninit<u8>> {
        type Ptr = Buffered<'a, T>;
        type Err = Infallible;

        fn emplace<C>(self, constructor: C) -> Result<Self::Ptr, Self::Err>
        where
            C: Construct<Object = T>,
        {
            unsafe {
                let layout = constructor.layout();
                let slot = vec_emplace(self, layout);
                let ptr = slot.as_ptr();

                let init = constructor.construct(slot);
                validate_slot(ptr, layout, init);
                Ok(Buffered::from_raw(init))
            }
        }
    }
    unsafe fn vec_emplace(vec: &mut Vec<MaybeUninit<u8>>, layout: Layout) -> Slot<'_> {
        if layout.size() == 0 {
            return dangling_slot(layout);
        }

        let mut buf = vec.as_mut_ptr();
        let mut align_offset = buf.align_offset(layout.align());
        let total_bytes = align_offset + layout.size();

        if total_bytes > vec.capacity() {
            vec.reserve(layout.size() + layout.align() - vec.len());
            buf = vec.as_mut_ptr();
            align_offset = buf.align_offset(layout.align());
        }
        let slot = buf.add(align_offset).cast::<u8>();
        Slot::new_unchecked(NonNull::new_unchecked(slot))
    }
}
#[cfg(feature = "alloc")]
pub use __alloc::*;

#[cfg(feature = "smallvec")]
mod __smallvec {
    use core::convert::Infallible;
    use core::mem::MaybeUninit;

    use smallvec::{Array, SmallVec};

    use super::*;

    #[cfg_attr(docsrs, doc(cfg(feature = "smallvec")))]
    unsafe impl<'a, A, T> Emplace<T> for &'a mut SmallVec<A>
    where
        A: Array<Item = MaybeUninit<u8>>,
        T: ?Sized,
    {
        type Ptr = Buffered<'a, T>;
        type Err = Infallible;

        fn emplace<C>(self, constructor: C) -> Result<Self::Ptr, Self::Err>
        where
            C: Construct<Object = T>,
        {
            unsafe {
                let layout = constructor.layout();
                let slot = small_vec_emplace(self, layout);
                let ptr = slot.as_ptr();

                let init = constructor.construct(slot);
                validate_slot(ptr, layout, init);
                Ok(Buffered::from_raw(init))
            }
        }
    }
    unsafe fn small_vec_emplace<A>(vec: &mut SmallVec<A>, layout: Layout) -> Slot<'_>
    where
        A: Array<Item = MaybeUninit<u8>>,
    {
        if layout.size() == 0 {
            return dangling_slot(layout);
        }

        let mut buf = vec.as_mut_ptr();
        let mut align_offset = buf.align_offset(layout.align());
        let total_bytes = align_offset + layout.size();

        if total_bytes > vec.capacity() {
            vec.reserve(layout.size() + layout.align() - vec.len());
            buf = vec.as_mut_ptr();
            align_offset = buf.align_offset(layout.align());
        }
        let slot = buf.add(align_offset).cast::<u8>();
        Slot::new_unchecked(NonNull::new_unchecked(slot))
    }
}

// TODO: is it possible to use strict provenance APIs?
unsafe fn dangling_slot<'a>(layout: Layout) -> Slot<'a> {
    Slot::new_unchecked(NonNull::new_unchecked(layout.align() as *mut u8))
}

fn validate_slot<T: ?Sized>(ptr: NonNull<u8>, layout: Layout, init: NonNull<T>) {
    if cfg!(debug_assertions) {
        let init_ptr = init.cast::<u8>();
        assert_eq!(init_ptr, ptr, "initialized address mismatches");
        let init_layout = unsafe { Layout::for_value(init.as_ref()) };
        assert_eq!(init_layout, layout, "initialized layout mismatches");
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "container_tests.rs"]
mod tests;
