use crate::container::{Container, PinContainer};
use core::alloc::Layout;
use core::pin::Pin;
use core::ptr::NonNull;

/// The main entrypoint used to perform in-place object constructions.
pub struct Dynify<T>(Initializer<T>);
impl<T> Dynify<T>
where
    T: Constructor,
{
    pub fn layout(&self) -> Layout {
        self.0.as_ref().layout()
    }

    pub fn init<C>(self, container: C) -> C::Ptr
    where
        C: Container<T::Object>,
    {
        self.try_init(container)
            .unwrap_or_else(|_| panic!("failed to initialize"))
    }

    pub fn try_init<C>(mut self, container: C) -> Result<C::Ptr, (Self, C::Err)>
    where
        C: Container<T::Object>,
    {
        unsafe {
            match container.emplace(self.0.as_mut()) {
                Ok(p) => {
                    core::mem::forget(self);
                    Ok(p)
                },
                Err(e) => Err((self, e)),
            }
        }
    }

    pub fn init2<P, C1, C2>(self, container1: C1, container2: C2) -> P
    where
        C1: Container<T::Object, Ptr = P>,
        C2: Container<T::Object, Ptr = P>,
    {
        self.try_init2(container1, container2)
            .unwrap_or_else(|_| panic!("failed to initialize"))
    }

    pub fn try_init2<P, C1, C2>(self, container1: C1, container2: C2) -> Result<P, (Self, C2::Err)>
    where
        C1: Container<T::Object, Ptr = P>,
        C2: Container<T::Object, Ptr = P>,
    {
        self.try_init(container1)
            .or_else(|(this, _)| this.try_init(container2))
    }

    #[cfg(feature = "alloc")]
    pub fn boxed(self) -> alloc::boxed::Box<T::Object> {
        self.init(crate::container::Boxed)
    }
}

/// A variant of [`Dynify`] that requires pinned containers.
pub struct PinDynify<T>(Initializer<T>);
impl<T: Constructor> PinDynify<T> {
    pub fn layout(&self) -> Layout {
        self.0.as_ref().layout()
    }

    pub fn init<C>(self, container: C) -> Pin<C::Ptr>
    where
        C: PinContainer<T::Object>,
    {
        self.try_init(container)
            .unwrap_or_else(|_| panic!("failed to initialize"))
    }

    pub fn try_init<C>(mut self, container: C) -> Result<Pin<C::Ptr>, (Self, C::Err)>
    where
        C: PinContainer<T::Object>,
    {
        unsafe {
            match container.pin_emplace(self.0.as_mut()) {
                Ok(p) => {
                    core::mem::forget(self);
                    Ok(p)
                },
                Err(e) => Err((self, e)),
            }
        }
    }

    pub fn init2<P, C1, C2>(self, container1: C1, container2: C2) -> Pin<P>
    where
        C1: PinContainer<T::Object, Ptr = P>,
        C2: PinContainer<T::Object, Ptr = P>,
    {
        self.try_init2(container1, container2)
            .unwrap_or_else(|_| panic!("failed to initialize"))
    }

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

    #[cfg(feature = "alloc")]
    pub fn boxed(self) -> Pin<alloc::boxed::Box<T::Object>> {
        self.init(crate::container::Boxed)
    }
}

/// The core type which packages necessary information to construct an object.
///
/// # Safety
///
/// The implementor must adhere to the documented contracts of each method.
pub unsafe trait Constructor: Sized {
    type Object: ?Sized;

    /// Returns the layout of the object to be constructed.
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
    /// The memory block that `slot` points to must meet the following
    /// requirements:
    ///
    /// - It satisfies the size and alignment constraints of the layout returned
    ///   from [`layout`](Self::layout).
    /// - It must be exclusive for this construction.
    ///
    /// [`Object`]: Self::Object
    unsafe fn construct(self, slot: Slot) -> NonNull<Self::Object>;

    /// Wraps the constructor with [`Dynify`] for further usage.
    fn dynify(self) -> Dynify<Self> {
        Dynify(Initializer::new(self))
    }

    /// Wraps the constructor with [`PinDynify`] to ensure it is constructed in
    /// pinned containers.
    fn pin_dynify(self) -> PinDynify<Self> {
        PinDynify(Initializer::new(self))
    }
}

/// A memory block used in to store arbitrary objects.
pub struct Slot(crate::VoidPtr);
impl Slot {
    /// Creates a new slot from the supplied pointer.
    ///
    /// # Safety
    ///
    /// - The returned [`Slot`] may not be used outside of [`construct`].
    /// - [`Constructor`]s will write objects directly to the address of `ptr`,
    ///   hence `ptr` must meet all safety requirements of [`construct`].
    ///
    /// [`construct`]: Constructor::construct
    pub unsafe fn new(ptr: NonNull<u8>) -> Self {
        Self(ptr.cast())
    }

    /// Consumes this slot and fills it with the supplied object.
    ///
    /// # Safety
    ///
    /// The object may not have a different layout than the one returned from
    /// [`Constructor::layout`].
    pub unsafe fn write<T>(self, object: T) -> NonNull<T> {
        let ptr = self.0.cast::<T>();
        debug_assert!(ptr.is_aligned());
        ptr.write(object);
        ptr
    }
}

/// A utility type that provides convenient methods for chained constructions.
///
/// # Example
///
/// ```rust
/// # use dynify::{Initializer, Container};
/// # use dynify::r#priv::I32Constructor;
/// # use std::any::Any;
/// let mut stack = [0u8; 32];
/// let mut heap = vec![0u8; 0];
/// let mut init = Initializer::new(I32Constructor);
/// let any;
/// // SAFETY: The constructor does not require pinned containers and will never
/// // be consumed twice.
/// if let Ok(p) = Container::emplace(&mut stack, unsafe { init.as_mut() }) {
///     any = p;
/// } else {
///     any = Container::emplace(&mut heap, unsafe { init.as_mut() }).expect("unreachable!")
/// };
/// assert!(any.downcast_ref::<i32>().is_some());
/// ```
pub struct Initializer<T>(Option<T>);
impl<T> Initializer<T> {
    /// Wraps the supplied constructor and returns a new instance.
    pub fn new(constructor: T) -> Self {
        Self(Some(constructor))
    }

    /// Returns a mutable reference to this initializer.
    ///
    /// # Safety
    ///
    /// For the returned reference:
    /// - It may not be used in non-pinned containers if the underlying
    ///   constructor requires pinned memory blocks.
    /// - After it is [`consume`]d, `self` must either be [`drop`]ed or be
    ///   [`forget`]ed immediately. Future access will lead to *undefined
    ///   behaviors*.
    ///
    /// [`consume`]: InitializerRefMut::consume
    /// [`forget`]: core::mem::forget
    pub unsafe fn as_mut(&mut self) -> InitializerRefMut<T> {
        InitializerRefMut(&mut self.0)
    }

    /// Returns an immutable reference to this initializer.
    pub fn as_ref(&self) -> InitializerRef<T> {
        let inner = unsafe { unwrap_unchecked(self.0.as_ref()) };
        InitializerRef(inner)
    }
}

/// An immutable reference to [`Initializer`].
pub struct InitializerRef<'a, T>(&'a T);
impl<T> core::ops::Deref for InitializerRef<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// A mutable reference to [`Initializer`].
pub struct InitializerRefMut<'a, T>(&'a mut Option<T>);
impl<T> InitializerRefMut<'_, T> {
    /// Consumes this initializer and returns the underlying constructor.
    pub fn consume(mut self) -> T {
        unsafe { self.consume_unchecked() }
    }

    /// Consumes this initializer without taking the ownship of `self`.
    pub(crate) unsafe fn consume_unchecked(&mut self) -> T {
        unsafe { unwrap_unchecked(self.0.take()) }
    }
}
impl<T> core::ops::Deref for InitializerRefMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { unwrap_unchecked(self.0.as_ref()) }
    }
}
impl<T> core::ops::DerefMut for InitializerRefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { unwrap_unchecked(self.0.as_mut()) }
    }
}

unsafe fn unwrap_unchecked<U>(opt: Option<U>) -> U {
    match opt {
        Some(t) => t,
        None => {
            #[cfg(debug_assertions)]
            panic!("Initializer has been consumed");
            #[cfg(not(debug_assertions))]
            core::hint::unreachable_unchecked();
        },
    }
}
