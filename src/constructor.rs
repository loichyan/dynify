use crate::container::{Container, PinContainer};
use core::alloc::Layout;
use core::marker::PhantomData;
use core::pin::Pin;
use core::ptr::NonNull;

pub struct Constructor<Dyn: ?Sized, Args> {
    layout: Layout,
    args: Args,
    init: unsafe fn(Slot, Args) -> NonNull<Dyn>,
}

pub unsafe fn new_constructor<Dyn: ?Sized, Args>(
    layout: Layout,
    args: Args,
    init: unsafe fn(Slot, Args) -> NonNull<Dyn>,
) -> Constructor<Dyn, Args> {
    Constructor { layout, args, init }
}

impl<Dyn: ?Sized, Args> Constructor<Dyn, Args> {
    pub fn layout(&self) -> Layout {
        self.layout
    }

    /// Constructs this object in the supplied slot.
    ///
    /// # Safety
    ///
    /// 1. `slot` must have enough space to fit the [`layout`] of this object.
    /// 2. `slot` must be exclusive for this construction.
    ///
    /// [`layout`]: Self::layout
    pub unsafe fn init_unchecked(self, slot: Slot) -> NonNull<Dyn> {
        (self.init)(slot, self.args)
    }

    pub fn init<C>(self, container: C) -> C::Ptr
    where
        C: Container<Dyn>,
    {
        container.emplace(self)
    }

    pub fn try_init<C>(self, container: C) -> Result<C::Ptr, C::Err<Args>>
    where
        C: Container<Dyn>,
    {
        container.try_emplace(self)
    }

    pub fn init2<P, C1, C2>(self, container1: C1, container2: C2) -> P
    where
        C1: Container<Dyn, Ptr = P, Err<Args> = Self>,
        C2: Container<Dyn, Ptr = P>,
    {
        container1
            .try_emplace(self)
            .unwrap_or_else(|this| container2.emplace(this))
    }

    pub fn try_init2<P, C1, C2>(self, container1: C1, container2: C2) -> Result<P, C2::Err<Args>>
    where
        C1: Container<Dyn, Ptr = P, Err<Args> = Self>,
        C2: Container<Dyn, Ptr = P>,
    {
        container1
            .try_emplace(self)
            .or_else(|this| container2.try_emplace(this))
    }

    #[cfg(feature = "alloc")]
    pub fn boxed(self) -> alloc::boxed::Box<Dyn> {
        self.init(crate::container::Boxed)
    }

    pub fn pinned(self) -> PinConstructor<Dyn, Args> {
        PinConstructor(self)
    }
}

/// A variant of [`Constructor`] that requires pinned containers.
pub struct PinConstructor<Dyn: ?Sized, Args>(Constructor<Dyn, Args>);
impl<Dyn: ?Sized, Args> PinConstructor<Dyn, Args> {
    pub fn layout(&self) -> Layout {
        self.0.layout()
    }

    pub fn init<C>(self, container: C) -> Pin<C::Ptr>
    where
        C: PinContainer<Dyn>,
    {
        container.pin_emplace(self.0)
    }

    pub fn try_init<C>(self, container: C) -> Result<Pin<C::Ptr>, C::PinErr<Args>>
    where
        C: PinContainer<Dyn>,
    {
        container.try_pin_emplace(self.0)
    }

    pub fn init2<P, C1, C2>(self, container1: C1, container2: C2) -> Pin<P>
    where
        C1: PinContainer<Dyn, Ptr = P, PinErr<Args> = Self>,
        C2: PinContainer<Dyn, Ptr = P>,
    {
        container1
            .try_pin_emplace(self.0)
            .unwrap_or_else(|this| container2.pin_emplace(this.0))
    }

    pub fn try_init2<P, C1, C2>(
        self,
        container1: C1,
        container2: C2,
    ) -> Result<Pin<P>, C2::PinErr<Args>>
    where
        C1: PinContainer<Dyn, Ptr = P, PinErr<Args> = Self>,
        C2: PinContainer<Dyn, Ptr = P>,
    {
        container1
            .try_pin_emplace(self.0)
            .or_else(|this| container2.try_pin_emplace(this.0))
    }

    #[cfg(feature = "alloc")]
    pub fn boxed(self) -> Pin<alloc::boxed::Box<Dyn>> {
        self.init(crate::container::Boxed)
    }

    pub unsafe fn unpinned(self) -> Constructor<Dyn, Args> {
        self.0
    }
}
impl<Dyn: ?Sized, Args> From<Constructor<Dyn, Args>> for PinConstructor<Dyn, Args> {
    fn from(value: Constructor<Dyn, Args>) -> Self {
        value.pinned()
    }
}

type VoidPtr = NonNull<Void>;
enum Void {}

pub struct Slot(VoidPtr);
impl Slot {
    pub unsafe fn new(ptr: NonNull<u8>) -> Self {
        Self(ptr.cast())
    }
    pub unsafe fn write<T>(self, val: T) -> NonNull<T> {
        self.0.cast().write(val);
        self.0.cast()
    }
}

pub struct Receiver<'a>(VoidPtr, PhantomData<&'a Void>);
impl<'a> Receiver<'a> {
    pub fn new<T>(data: &'a T) -> Self {
        Self(NonNull::from(data).cast(), PhantomData)
    }
    pub unsafe fn get<T>(self) -> &'a T {
        self.0.cast().as_ref()
    }
}

pub struct ReceiverMut<'a>(VoidPtr, PhantomData<&'a mut Void>);
impl<'a> ReceiverMut<'a> {
    pub fn new<T>(data: &'a mut T) -> Self {
        Self(NonNull::from(data).cast(), PhantomData)
    }
    pub unsafe fn get<T>(self) -> &'a mut T {
        self.0.cast().as_mut()
    }
}
