use core::alloc::Layout;
use core::fmt;
use core::marker::PhantomData;
use core::pin::Pin;
use core::ptr::NonNull;

use crate::container::{Emplace, PinEmplace};
use crate::utils::Void;

/// The core trait to package necessary information for object constructions.
///
/// A type that implements [`Construct`] is called a *constructor*. The value to
/// be constructed in the target memory location is called an *object*, of which
/// type is specified as [`Object`].
///
/// # Examples
///
/// ```rust
/// # use dynify::{Construct, PinConstruct, Slot};
/// # use std::alloc::Layout;
/// # use std::any::Any;
/// # use std::ptr::NonNull;
/// struct I32Construct(fn() -> i32);
/// unsafe impl PinConstruct for I32Construct {
///     type Object = dyn Any;
///     fn layout(&self) -> Layout {
///         Layout::new::<i32>()
///     }
///     unsafe fn construct(self, slot: Slot) -> NonNull<Self::Object> {
///         slot.write_unchecked((self.0)()) as NonNull<_>
///     }
/// }
/// unsafe impl Construct for I32Construct {}
/// ```
///
/// # Safety
///
/// For the implementor,
///
/// - It must adhere to the documented contracts of each method.
/// - The object placed in the provided slot in [`construct`] must have the same
///   layout as that returned from [`layout`].
///
/// [`Object`]: Self::Object
/// [`construct`]: Self::construct
/// [`layout`]: Self::layout
pub unsafe trait PinConstruct: Sized {
    /// The type of objects to be constructed.
    type Object: ?Sized;

    /// Returns the layout of the object to be constructed.
    ///
    /// [`Object`] can be a sized or unsize type. In the former case, this
    /// returns its layout. While in the latter case, this returns the layout of
    /// the original type of the coerced DST.
    ///
    /// [`Object`]: Self::Object
    fn layout(&self) -> Layout;

    /// Constructs the object in the specified address.
    ///
    /// This function will write the object to `slot` and therefore overwrite
    /// existing data at the address of `slot`. For the returned pointer, the
    /// following statements are always true:
    ///
    /// - It has the same address as the memory block owned by `slot`.
    /// - It may contain additional metadata if [`Object`] is unsized.
    /// - Invoking [`Layout::for_value`] with the deference of it returns a
    ///   layout that matches the one from [`Self::layout`] exactly.
    ///
    /// # Safety
    ///
    /// The memory block owned by `slot` must meet the following requirements:
    ///
    /// - It satisfies the size and alignment constraints of the layout returned
    ///   from [`Self::layout`].
    /// - It must be exclusive for this construction.
    ///
    /// Furthermore, if `Self` does not implement [`Construct`], the caller
    /// must ensure the pinning requirements are upheld for the returned
    /// pointer.
    ///
    /// [`Object`]: Self::Object
    unsafe fn construct(self, slot: Slot) -> NonNull<Self::Object>;
}

/// A marker for constructors that do not require pinned containers.
///
/// # Safety
///
/// See the safety notes of [`PinConstruct`]. Additionally, the implementor must
/// ensure that the implementation of [`construct`] does not rely on a pinned
/// memory block.
///
/// [`construct`]: PinConstruct::construct
pub unsafe trait Construct: PinConstruct {}

unsafe impl<T: PinConstruct> PinConstruct for &'_ mut Option<T> {
    type Object = T::Object;
    fn layout(&self) -> Layout {
        self.as_ref()
            .expect("constructor has been consumed")
            .layout()
    }
    unsafe fn construct(self, slot: Slot) -> NonNull<Self::Object> {
        self.take()
            .expect("constructor has been consumed")
            .construct(slot)
    }
}
unsafe impl<T: Construct> Construct for &'_ mut Option<T> {}

/// A memory block used to store arbitrary objects.
#[must_use = "slot must be consumed"]
pub struct Slot<'a, T: ?Sized = Void>(NonNull<T>, PhantomData<&'a mut T>);
impl<'a> Slot<'a> {
    /// Creates a new slot from the supplied pointer.
    ///
    /// # Safety
    ///
    /// - The returned instance may not be used outside of [`construct`].
    /// - [`Construct`]s will write objects directly to the address of `ptr`,
    ///   hence `ptr` must meet all safety requirements of [`construct`].
    ///
    /// [`construct`]: PinConstruct::construct
    pub unsafe fn new_unchecked(ptr: NonNull<u8>) -> Self {
        Self(ptr.cast(), PhantomData)
    }

    /// Consumes this slot, filling it with the supplied object.
    ///
    /// # Safety
    ///
    /// The object may not have a different layout than the one returned from
    /// [`PinConstruct::layout`].
    pub unsafe fn write_unchecked<T>(self, object: T) -> NonNull<T> {
        let ptr = self.0.cast::<T>();
        debug_assert!(ptr.is_aligned());
        ptr.write(object);
        ptr
    }

    /// Transforms this slot into a typed one.
    ///
    /// # Safety
    ///
    /// The layout of `T` must match that from [`PinConstruct::layout`] exactly.
    pub unsafe fn cast<T>(self) -> Slot<'a, T> {
        Slot(self.0.cast(), PhantomData)
    }
}
impl<'a, T> Slot<'a, T> {
    /// Consumes this slot, filling it with the supplied object.
    ///
    /// The returned object is sealed in [`Opaque`] to prevent misuse. Despite
    /// this, as demonstrated below, it's still possible to coerce it into a
    /// trait object:
    ///
    /// ```rust
    /// # use dynify::{Opaque, Slot};
    /// # use std::any::Any;
    /// fn fill_slot(slot: Slot<String>) -> &mut Opaque<dyn Any> {
    ///     slot.write(String::from("WRYYY!")) as &mut Opaque<dyn Any>
    /// }
    /// ```
    pub fn write(self, object: T) -> &'a mut Opaque<T> {
        unsafe {
            Slot::new_unchecked(self.into_raw())
                .write_unchecked(Opaque(object))
                .as_mut()
        }
    }
}
impl<'a, T: ?Sized> Slot<'a, T> {
    /// Consumes this instance, returning a raw pointer to the allocated memory
    /// block.
    pub fn into_raw(self) -> NonNull<u8> {
        self.0.cast()
    }

    /// Returns a raw pointer to the allocated memory block.
    pub(crate) fn as_ptr(&self) -> NonNull<u8> {
        self.0.cast()
    }
}

impl<T: ?Sized> fmt::Debug for Slot<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A opaque wrapper of initialized objects.
///
/// This structure has the same in-memory representation as its inner type `T`.
/// Therefore, it is possible, although highly discouraged, to [`transmute`]
/// between `Opaque<T>` and `T`.
///
/// [`transmute`]: core::mem::transmute
#[repr(transparent)]
pub struct Opaque<T: ?Sized>(T);
impl<T: ?Sized> Opaque<T> {
    pub(crate) fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// The main interface used to perform in-place object constructions.
pub trait Dynify: Construct {
    /// Constructs the object in the supplied container.
    ///
    /// For a non-panicking alternative see [`try_init`](Self::try_init).
    ///
    /// # Panic
    ///
    /// It panics if `container` fails to construct the object.
    fn init<C>(self, container: C) -> C::Ptr
    where
        C: Emplace<Self::Object>,
    {
        self.try_init(container)
            .unwrap_or_else(|_| panic!("failed to initialize"))
    }

    /// Constructs the object in the supplied container.
    ///
    /// If the construction succeeds, it returns the pointer to the object.
    /// Otherwise, `self` is returned along with the encountered error.
    fn try_init<C>(self, container: C) -> Result<C::Ptr, (Self, C::Err)>
    where
        C: Emplace<Self::Object>,
    {
        let mut fallible = FallibleConstructor::new(self);
        // SAFETY: `fallible` is dropped immediately after it gets consumed.
        let handle = unsafe { fallible.handle() };
        match container.emplace(handle) {
            Ok(p) => {
                debug_assert!(fallible.consumed());
                core::mem::forget(fallible);
                Ok(p)
            },
            Err(e) => Err((fallible.into_inner(), e)),
        }
    }

    /// Constructs the object in two containers in turn.
    ///
    /// For a non-panicking alternative see [`try_init2`](Self::try_init2).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dynify::{from_fn, Dynify, Fn};
    /// # use std::future::Future;
    /// # use std::mem::MaybeUninit;
    /// # pollster::block_on(async {
    /// let mut stack = MaybeUninit::<[u8; 16]>::uninit();
    /// let mut heap = Vec::<MaybeUninit<u8>>::new();
    ///
    /// let constructor: Fn!(=> dyn Future<Output = i32>) = from_fn!(|| async { 777 });
    /// let ret = constructor.init2(&mut stack, &mut heap).await;
    /// assert_eq!(ret, 777);
    /// # });
    /// ```
    ///
    /// # Panic
    ///
    /// It panics if both containers fail to construct the object.
    fn init2<P, C1, C2>(self, container1: C1, container2: C2) -> P
    where
        C1: Emplace<Self::Object, Ptr = P>,
        C2: Emplace<Self::Object, Ptr = P>,
    {
        self.try_init2(container1, container2)
            .unwrap_or_else(|_| panic!("failed to initialize"))
    }

    /// Constructs the object in two containers in turn.
    ///
    /// It returns the object pointer if either container succeeds. Otherwise,
    /// it forwards the error returned from `container2`.
    fn try_init2<P, C1, C2>(self, container1: C1, container2: C2) -> Result<P, (Self, C2::Err)>
    where
        C1: Emplace<Self::Object, Ptr = P>,
        C2: Emplace<Self::Object, Ptr = P>,
    {
        self.try_init(container1)
            .or_else(|(this, _)| this.try_init(container2))
    }

    /// Constructs the object in [`Box`](alloc::boxed::Box).
    ///
    /// This function never fails as long as there is enough free memory.
    #[cfg(feature = "alloc")]
    fn boxed(self) -> alloc::boxed::Box<Self::Object> {
        self.init(crate::container::Boxed)
    }
}
impl<T: Construct> Dynify for T {}

/// A variant of [`Dynify`] that requires pinned containers.
pub trait PinDynify: PinConstruct {
    /// Constructs the object in the supplied container.
    ///
    /// For a non-panicking alternative see [`try_pin_init`](Self::try_pin_init).
    ///
    /// # Panic
    ///
    /// It panics if `container` fails to construct the object.
    fn pin_init<C>(self, container: C) -> Pin<C::Ptr>
    where
        C: PinEmplace<Self::Object>,
    {
        self.try_pin_init(container)
            .unwrap_or_else(|_| panic!("failed to initialize"))
    }

    /// Constructs the object in the supplied container.
    ///
    /// If the construction succeeds, it returns the pointer to the object.
    /// Otherwise, `self` is returned along with the encountered error.
    fn try_pin_init<C>(self, container: C) -> Result<Pin<C::Ptr>, (Self, C::Err)>
    where
        C: PinEmplace<Self::Object>,
    {
        let mut fallible = FallibleConstructor::new(self);
        // SAFETY: `fallible` is dropped immediately after it gets consumed.
        let handle = unsafe { fallible.handle() };
        match container.pin_emplace(handle) {
            Ok(p) => {
                debug_assert!(fallible.consumed());
                core::mem::forget(fallible);
                Ok(p)
            },
            Err(e) => Err((fallible.into_inner(), e)),
        }
    }

    /// Constructs the object in two containers in turn.
    ///
    /// For a non-panicking alternative see [`try_pin_init2`](Self::try_pin_init2).
    ///
    /// # Panic
    ///
    /// It panics if both containers fail to construct the object.
    fn pin_init2<P, C1, C2>(self, container1: C1, container2: C2) -> Pin<P>
    where
        C1: PinEmplace<Self::Object, Ptr = P>,
        C2: PinEmplace<Self::Object, Ptr = P>,
    {
        self.try_pin_init2(container1, container2)
            .unwrap_or_else(|_| panic!("failed to initialize"))
    }

    /// Constructs the object in two containers in turn.
    ///
    /// It returns the object pointer if either container succeeds. Otherwise,
    /// it forwards the error returned from `container2`.
    fn try_pin_init2<P, C1, C2>(
        self,
        container1: C1,
        container2: C2,
    ) -> Result<Pin<P>, (Self, C2::Err)>
    where
        C1: PinEmplace<Self::Object, Ptr = P>,
        C2: PinEmplace<Self::Object, Ptr = P>,
    {
        self.try_pin_init(container1)
            .or_else(|(this, _)| this.try_pin_init(container2))
    }

    /// Constructs the object in [`Box`](alloc::boxed::Box).
    ///
    /// This function never fails as long as there is enough free memory.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dynify::{from_fn, Fn, PinDynify};
    /// # use std::any::Any;
    /// # use std::pin::Pin;
    /// let constructor: Fn!(=> dyn Any) = from_fn!(|| 123);
    /// let _: Pin<Box<dyn Any>> = constructor.pin_boxed();
    /// ```
    #[cfg(feature = "alloc")]
    fn pin_boxed(self) -> Pin<alloc::boxed::Box<Self::Object>> {
        self.pin_init(crate::container::Boxed)
    }
}
impl<T: PinConstruct> PinDynify for T {}

/// A utility type to reuse the inner constructor if construction fails.
struct FallibleConstructor<T>(Option<T>);
impl<T> FallibleConstructor<T> {
    /// Wraps the supplied constructor and returns a new instance.
    pub fn new(constructor: T) -> Self {
        Self(Some(constructor))
    }

    /// Returns whether the inner constructor is consumed.
    pub fn consumed(&self) -> bool {
        self.0.is_none()
    }

    /// Consumes this instance, returning the inner constructor.
    pub fn into_inner(self) -> T {
        debug_assert!(!self.consumed());
        unwrap_unchecked(self.0)
    }

    /// Returns a handle for fallible construction.
    ///
    /// # Safety
    ///
    /// For the returned handle:
    ///
    /// - It may not be used in non-pinned containers if the underlying
    ///   constructor requires pinned memory blocks.
    /// - After it is [`construct`]ed, `self` must either be [`drop`]ed or
    ///   [`forget`]ed immediately. Future access will lead to *undefined
    ///   behavior*.
    ///
    /// [`construct`]: PinConstruct::construct
    /// [`forget`]: core::mem::forget
    pub unsafe fn handle(&mut self) -> FallibleHandle<'_, T> {
        debug_assert!(!self.consumed());
        FallibleHandle(&mut self.0)
    }
}

/// The handle to perform fallible constructions.
///
/// If it is not consumed through [`construct`], the inner constructor remains
/// valid. Otherwise, the inner value gets taken, leading to *undefined
/// behavior* for future access.
///
/// [`construct`]: PinConstruct::construct
struct FallibleHandle<'a, T>(&'a mut Option<T>);
unsafe impl<T: PinConstruct> PinConstruct for FallibleHandle<'_, T> {
    type Object = T::Object;
    fn layout(&self) -> Layout {
        unwrap_unchecked(self.0.as_ref()).layout()
    }
    unsafe fn construct(self, slot: Slot) -> NonNull<Self::Object> {
        unwrap_unchecked(self.0.take()).construct(slot)
    }
}
unsafe impl<T: Construct> Construct for FallibleHandle<'_, T> {}

fn unwrap_unchecked<U>(opt: Option<U>) -> U {
    match opt {
        Some(t) => t,
        // SAFETY: The validity of the constructor inside `Option` is guaranteed
        // by the caller.
        None => unsafe { core::hint::unreachable_unchecked() },
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "constructor_tests.rs"]
mod tests;
