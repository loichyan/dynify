use core::alloc::Layout;
use core::pin::Pin;
use core::ptr::NonNull;

use crate::container::{Container, PinContainer};

/// The main entrypoint used to perform in-place object constructions.
pub struct Dynify<T>(T);
impl<T> Dynify<T>
where
    T: Constructor,
{
    /// Returns the layout of the object to be constructed.
    pub fn layout(&self) -> Layout {
        self.0.layout()
    }

    /// Constructs the object in the supplied container.
    ///
    /// For non-panicking variant, use [`try_init`](Self::try_init).
    ///
    /// # Panic
    ///
    /// It panics if `container` fails to construct the object.
    pub fn init<C>(self, container: C) -> C::Ptr
    where
        C: Container<T::Object>,
    {
        self.try_init(container)
            .unwrap_or_else(|_| panic!("failed to initialize"))
    }

    /// Constructs the object in the supplied container.
    ///
    /// If the construction succeeds, it returns the pointer to the object.
    /// Otherwise, `self` is returned along with the encountered error.
    pub fn try_init<C>(self, container: C) -> Result<C::Ptr, (Self, C::Err)>
    where
        C: Container<T::Object>,
    {
        let mut fallible = FallibleConstructor::new(self.0);
        // SAFETY: `fallible` is dropped immediately after it gets consumed.
        let handle = unsafe { fallible.handle() };
        match container.emplace(handle) {
            Ok(p) => {
                debug_assert!(fallible.consumed());
                core::mem::forget(fallible);
                Ok(p)
            },
            Err(e) => Err((Self(fallible.into_inner()), e)),
        }
    }

    /// Constructs the object in two containers in turn.
    ///
    /// For non-panicking variant, use [`try_init2`](Self::try_init2).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dynify::{Dynify, Fn, from_fn};
    /// # use std::future::Future;
    /// # pollster::block_on(async {
    /// let mut stack = [0u8; 32];
    /// let mut heap = vec![0u8; 0];
    ///
    /// let constructor: Dynify<Fn!(=> dyn Future<Output = i32>)> = from_fn!(|| async { 777 });
    /// let ret = constructor.init2(&mut stack, &mut heap).await;
    /// assert_eq!(ret, 777);
    /// # });
    /// ```
    ///
    /// # Panic
    ///
    /// It panics if both containers fail to construct the object.
    pub fn init2<P, C1, C2>(self, container1: C1, container2: C2) -> P
    where
        C1: Container<T::Object, Ptr = P>,
        C2: Container<T::Object, Ptr = P>,
    {
        self.try_init2(container1, container2)
            .unwrap_or_else(|_| panic!("failed to initialize"))
    }

    /// Constructs the object in two containers in turn.
    ///
    /// It returns the object pointer if either container succeeds. Otherwise,
    /// it forwards the error returned from `container2`.
    pub fn try_init2<P, C1, C2>(self, container1: C1, container2: C2) -> Result<P, (Self, C2::Err)>
    where
        C1: Container<T::Object, Ptr = P>,
        C2: Container<T::Object, Ptr = P>,
    {
        self.try_init(container1)
            .or_else(|(this, _)| this.try_init(container2))
    }

    /// Constructs the object in [`Box`](alloc::boxed::Box).
    ///
    /// This function never fails as long as there is enough free memory.
    #[cfg(feature = "alloc")]
    pub fn boxed(self) -> alloc::boxed::Box<T::Object> {
        self.init(crate::container::Boxed)
    }
}

/// A variant of [`Dynify`] that requires pinned containers.
pub struct PinDynify<T>(T);
impl<T: PinConstructor> PinDynify<T> {
    /// Returns the layout of the object to be constructed.
    pub fn layout(&self) -> Layout {
        self.0.layout()
    }

    /// Constructs the object in the supplied container.
    ///
    /// For non-panicking variant, use [`try_init`](Self::try_init).
    ///
    /// # Panic
    ///
    /// It panics if `container` fails to construct the object.
    pub fn init<C>(self, container: C) -> Pin<C::Ptr>
    where
        C: PinContainer<T::Object>,
    {
        self.try_init(container)
            .unwrap_or_else(|_| panic!("failed to initialize"))
    }

    /// Constructs the object in the supplied container.
    ///
    /// If the construction succeeds, it returns the pointer to the object.
    /// Otherwise, `self` is returned along with the encountered error.
    pub fn try_init<C>(self, container: C) -> Result<Pin<C::Ptr>, (Self, C::Err)>
    where
        C: PinContainer<T::Object>,
    {
        let mut fallible = FallibleConstructor::new(self.0);
        // SAFETY: `fallible` is dropped immediately after it gets consumed.
        let handle = unsafe { fallible.handle() };
        match container.pin_emplace(handle) {
            Ok(p) => {
                debug_assert!(fallible.consumed());
                core::mem::forget(fallible);
                Ok(p)
            },
            Err(e) => Err((Self(fallible.into_inner()), e)),
        }
    }

    /// Constructs the object in two containers in turn.
    ///
    /// For non-panicking variant, use [`try_init2`](Self::try_init2).
    ///
    /// # Panic
    ///
    /// It panics if both containers fail to construct the object.
    pub fn init2<P, C1, C2>(self, container1: C1, container2: C2) -> Pin<P>
    where
        C1: PinContainer<T::Object, Ptr = P>,
        C2: PinContainer<T::Object, Ptr = P>,
    {
        self.try_init2(container1, container2)
            .unwrap_or_else(|_| panic!("failed to initialize"))
    }

    /// Constructs the object in two containers in turn.
    ///
    /// It returns the object pointer if either container succeeds. Otherwise,
    /// it forwards the error returned from `container2`.
    pub fn try_init2<P, C1, C2>(
        self,
        container1: C1,
        container2: C2,
    ) -> Result<Pin<P>, (Self, C2::Err)>
    where
        C1: PinContainer<T::Object, Ptr = P>,
        C2: PinContainer<T::Object, Ptr = P>,
    {
        self.try_init(container1)
            .or_else(|(this, _)| this.try_init(container2))
    }

    /// Constructs the object in [`Box`](alloc::boxed::Box).
    ///
    /// This function never fails as long as there is enough free memory.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dynify::{Fn, PinDynify, from_fn};
    /// # use std::any::Any;
    /// # use std::pin::Pin;
    /// let constructor: PinDynify<Fn!(=> dyn Any)> = from_fn!(|| 123);
    /// let _: Pin<Box<dyn Any>> = constructor.boxed();
    /// ```
    #[cfg(feature = "alloc")]
    pub fn boxed(self) -> Pin<alloc::boxed::Box<T::Object>> {
        self.init(crate::container::Boxed)
    }
}

/// The core trait to package necessary information for object constructions.
///
/// # Examples
///
/// ```rust
/// # use dynify::{Constructor, PinConstructor, Slot};
/// # use std::any::Any;
/// # use std::alloc::Layout;
/// # use std::ptr::NonNull;
/// struct I32Constructor(fn() -> i32);
/// unsafe impl PinConstructor for I32Constructor {
///     type Object = dyn Any;
///     fn layout(&self) -> Layout {
///         Layout::new::<i32>()
///     }
///     unsafe fn construct(self, slot: Slot) -> NonNull<Self::Object> {
///         slot.write((self.0)()) as NonNull<_>
///     }
/// }
/// unsafe impl Constructor for I32Constructor {}
/// ```
///
/// # Safety
///
/// The implementor must adhere to the documented contracts of each method.
pub unsafe trait PinConstructor: Sized {
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
    /// existing data at the address of `slot`. The returned pointer have the
    /// same address as `slot` and may contain additional metadata if [`Object`]
    /// is unsized.
    ///
    /// # Safety
    ///
    /// The memory block owned by `slot` must meet the following requirements:
    ///
    /// - It satisfies the size and alignment constraints of the layout returned
    ///   from [`layout`](Self::layout).
    /// - It must be exclusive for this construction.
    ///
    /// Furthermore, if `Self` does not implement [`Constructor`], the caller
    /// must ensure the pinning requirements are upheld for the returned
    /// pointer.
    ///
    /// [`Object`]: Self::Object
    unsafe fn construct(self, slot: Slot) -> NonNull<Self::Object>;

    /// Wraps the constructor with [`PinDynify`] to ensure it is constructed in
    /// pinned containers.
    fn pin_dynify(self) -> PinDynify<Self> {
        PinDynify(self)
    }
}

/// A marker for constructors that do not require to be pinned.
///
/// # Safety
///
/// The implementor must ensure that the implementation of [`construct`] does
/// not rely on a pinned slot.
///
/// [`construct`]: PinConstructor::construct
pub unsafe trait Constructor: PinConstructor {
    /// Wraps the constructor with [`Dynify`] for further use.
    fn dynify(self) -> Dynify<Self> {
        Dynify(self)
    }
}

/// A memory block used to store arbitrary objects.
pub struct Slot(crate::VoidPtr);
impl Slot {
    /// Creates a new slot from the supplied pointer.
    ///
    /// # Safety
    ///
    /// - The returned instance may not be used outside of [`construct`].
    /// - [`Constructor`]s will write objects directly to the address of `ptr`,
    ///   hence `ptr` must meet all safety requirements of [`construct`].
    ///
    /// [`construct`]: PinConstructor::construct
    pub unsafe fn new(ptr: NonNull<u8>) -> Self {
        Self(ptr.cast())
    }

    /// Consumes this slot and fills it with the supplied object.
    ///
    /// # Safety
    ///
    /// The object may not have a different layout than the one returned from
    /// [`PinConstructor::layout`].
    pub unsafe fn write<T>(self, object: T) -> NonNull<T> {
        let ptr = self.0.cast::<T>();
        debug_assert!(ptr.is_aligned());
        ptr.write(object);
        ptr
    }
}

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
    /// [`construct`]: PinConstructor::construct
    /// [`forget`]: core::mem::forget
    pub unsafe fn handle(&mut self) -> FallibleHandle<T> {
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
/// [`construct`]: Constructor::construct
struct FallibleHandle<'a, T>(&'a mut Option<T>);
unsafe impl<T: PinConstructor> PinConstructor for FallibleHandle<'_, T> {
    type Object = T::Object;
    fn layout(&self) -> Layout {
        unwrap_unchecked(self.0.as_ref()).layout()
    }
    unsafe fn construct(self, slot: Slot) -> NonNull<Self::Object> {
        unwrap_unchecked(self.0.take()).construct(slot)
    }
}
unsafe impl<T: Constructor> Constructor for FallibleHandle<'_, T> {}

fn unwrap_unchecked<U>(opt: Option<U>) -> U {
    match opt {
        Some(t) => t,
        // SAFETY: The validity of the constructor inside `Option` is guaranteed
        // by the caller.
        None => unsafe { core::hint::unreachable_unchecked() },
    }
}
