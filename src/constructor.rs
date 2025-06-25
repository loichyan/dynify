use crate::container::{Container, PinContainer};
use core::alloc::Layout;
use core::pin::Pin;
use core::ptr::NonNull;

/// [`Dynify`] provides safe APIs to perform in-place object constructions.
pub struct Dynify<T>(Initializer<T>);
impl<T> Dynify<T>
where
    T: Constructor,
{
    pub fn layout(&self) -> Layout {
        self.0.layout()
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
        match container.emplace(&mut self.0) {
            Ok(p) => {
                core::mem::forget(self);
                Ok(p)
            },
            Err(e) => Err((self, e)),
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
        self.0.layout()
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
        match container.pin_emplace(&mut self.0) {
            Ok(p) => {
                core::mem::forget(self);
                Ok(p)
            },
            Err(e) => Err((self, e)),
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

/// A [`Constructor`] packages necessary information to construct an object.
///
/// # Safety
///
/// If the object requires to be constructed in a pinned memory block, it must
/// be ensured that the constructor cannot be used with a non-pinned container
/// in safe Rust. Namely, it may only be used with [`PinContainer`].
///
/// [`layout`]: Self::layout
pub unsafe trait Constructor: Sized {
    type Object: ?Sized;

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

    /// Returns the layout of the object to be constructed.
    fn layout(&self) -> Layout;

    /// Wraps the constructor in [`Dynify`] for further usage.
    fn dynify(self) -> Dynify<Self> {
        Dynify(unsafe { Initializer::new(self) })
    }

    /// Wraps the constructor in [`PinDynify`] to ensure it is constructed in
    /// pinned containers.
    fn pin_dynify(self) -> PinDynify<Self> {
        PinDynify(unsafe { Initializer::new(self) })
    }
}

pub struct Initializer<T>(Option<T>);
impl<T> Initializer<T> {
    pub unsafe fn new(constructor: T) -> Self {
        Self(Some(constructor))
    }

    pub fn layout(&self) -> Layout
    where
        T: Constructor,
    {
        unsafe { Self::unwrap_unchecked(self.0.as_ref()).layout() }
    }

    pub unsafe fn init_unchecked(&mut self, slot: Slot) -> NonNull<T::Object>
    where
        T: Constructor,
    {
        Self::unwrap_unchecked(self.0.take()).construct(slot)
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
}

/// A slot used in [`Constructor::construct`] to store arbitrary objects.
pub struct Slot(crate::VoidPtr);
impl Slot {
    /// Creates a new slot from the supplied pointer.
    ///
    /// # Safety
    ///
    /// The returned [`Slot`] may not be used outside of [`construct`].
    /// [`Constructor`]s will write objects directly to the address of `ptr`,
    /// hence `ptr` must meet all requirements of [`construct`].
    ///
    /// [`construct`]: Constructor::construct.
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
