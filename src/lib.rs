#![allow(async_fn_in_trait)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::missing_safety_doc)]

use std::alloc::Layout;
use std::convert::Infallible;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::ptr::NonNull;

// =========== Retrieve return type from arbitrary functions ===========
mod function {
    use super::*;
    use std::any::Any;

    pub trait Function<Input> {
        type Output;
    }

    macro_rules! impl_function {
        ($($i:ident),* -> $o:ident) => {
            impl<Fn, $($i,)* $o> Function<($($i,)*)> for Fn
            where
                Fn: FnOnce($($i,)*) -> $o,
            {
                type Output = $o;
            }
        };
    }
    impl_function!(A                -> R);
    impl_function!(A, B             -> R);
    impl_function!(A, B, C          -> R);
    impl_function!(A, B, C, D       -> R);
    impl_function!(A, B, C, D, E    -> R);
    impl_function!(A, B, C, D, E, F -> R);

    pub const fn return_type_layout<I, F: Function<I>>(_: &F) -> Layout {
        Layout::new::<F::Output>()
    }

    pub const fn return_type_cast_ptr<I, F: Function<I>>(
        _: &F,
        ptr: VoidPtr,
    ) -> NonNull<F::Output> {
        ptr.cast()
    }

    pub fn test_return_type_layout() {
        fn f1(_: usize, _: usize) -> usize {
            todo!()
        }
        fn f2(_: &str) -> &str {
            todo!()
        }
        fn f3(_: String, _: Vec<u8>, _: &dyn Any) -> Box<dyn Any> {
            todo!()
        }
        fn f4(_: usize, _: usize) -> Infallible {
            todo!()
        }

        assert_eq!(Layout::new::<usize>(), return_type_layout(&f1));
        assert_eq!(Layout::new::<&str>(), return_type_layout(&f2));
        assert_eq!(Layout::new::<Box<dyn Any>>(), return_type_layout(&f3));
        assert_eq!(Layout::new::<Infallible>(), return_type_layout(&f4));
        println!("test_return_type_layout pass");
    }
}
pub use function::*;

// =========== Construct `dyn` object in arbitaray containers ===========
mod dyn_init {
    pub use super::*;
    use std::ops::{Deref, DerefMut};

    pub struct Constructor<Dyn: ?Sized, Args> {
        layout: Layout,
        args: Args,
        init: unsafe fn(VoidPtr, Args) -> NonNull<Dyn>,
    }

    pub type VoidPtr = NonNull<Void>;
    pub enum Void {}

    impl<Dyn: ?Sized, Args> Constructor<Dyn, Args> {
        pub unsafe fn new(
            layout: Layout,
            args: Args,
            init: unsafe fn(VoidPtr, Args) -> NonNull<Dyn>,
        ) -> Self {
            Self { layout, args, init }
        }

        pub fn layout(&self) -> Layout {
            self.layout
        }

        /// Constructs the `dyn` object in the supplied slot.
        ///
        /// # Safety
        ///
        /// 1. `slot` must have enough space to fit the [`layout`] of the object.
        /// 2. `slot` must be exclusive for this construction.
        ///
        /// [`layout`]: Self::layout
        pub unsafe fn construct(self, slot: VoidPtr) -> NonNull<Dyn> {
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

        pub fn boxed(self) -> Box<Dyn> {
            self.init(Boxed)
        }

        pub fn buffered(self, buf: &mut [u8]) -> Buffered<Dyn> {
            self.init(buf)
        }

        pub fn try_buffered(self, buf: &mut [u8]) -> Result<Buffered<Dyn>, Self> {
            self.try_init(buf)
        }

        pub fn pinned(self) -> PinConstructor<Dyn, Args> {
            PinConstructor(self)
        }
    }

    /// A variant of [`Constructor`] that requires pinned pointers.
    pub struct PinConstructor<Dyn: ?Sized, Args>(Constructor<Dyn, Args>);
    impl<Dyn: ?Sized, Args> PinConstructor<Dyn, Args> {
        pub fn boxed(self) -> Pin<Box<Dyn>> {
            Box::into_pin(self.0.boxed())
        }

        pub fn buffered(self, buf: Pin<&mut [u8]>) -> Pin<Buffered<Dyn>> {
            self.0.init(buf)
        }

        pub fn try_buffered(self, buf: Pin<&mut [u8]>) -> Result<Pin<Buffered<Dyn>>, Self> {
            self.0.try_init(buf).map_err(Self)
        }

        pub fn unpinned(self) -> Constructor<Dyn, Args> {
            self.0
        }
    }

    /// A one-time container used to construct `dyn` objects.
    pub unsafe trait Container<Dyn: ?Sized>: Sized {
        type Ptr;
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
                _ => unsafe { NonNull::new(std::alloc::alloc(layout)) }
                    .unwrap_or_else(|| std::alloc::handle_alloc_error(layout)),
            };
            unsafe {
                let ptr = constructor.construct(slot.cast());
                Ok(Box::from_raw(ptr.as_ptr()))
            }
        }
    }

    pub struct Buffered<'a, Dyn: ?Sized>(NonNull<Dyn>, PhantomData<&'a mut [u8]>);
    impl<Dyn: ?Sized> Drop for Buffered<'_, Dyn> {
        fn drop(&mut self) {
            unsafe { self.0.as_ptr().drop_in_place() }
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
    unsafe impl<'a, Dyn: ?Sized> Container<Dyn> for &'a mut [u8] {
        type Ptr = Buffered<'a, Dyn>;
        type Err<Args> = Constructor<Dyn, Args>;

        fn try_emplace<Args>(
            self,
            constructor: Constructor<Dyn, Args>,
        ) -> Result<Self::Ptr, Self::Err<Args>> {
            let layout = constructor.layout();
            let capacity = self.len();

            let buf = self as *mut [u8] as *mut u8;
            let buf_end = buf.wrapping_add(capacity);
            let slot = buf.wrapping_add(buf.align_offset(layout.align()));
            let slot_end = slot.wrapping_add(layout.size());

            if slot_end > buf_end || buf < slot {
                return Err(constructor);
            }
            unsafe {
                let ptr = constructor.construct(NonNull::new_unchecked(slot).cast());
                Ok(Buffered(ptr, PhantomData))
            }
        }
    }

    // Pinned buffer
    unsafe impl<'a, Dyn: ?Sized> Container<Dyn> for Pin<&'a mut [u8]> {
        type Ptr = Pin<Buffered<'a, Dyn>>;
        type Err<Args> = Constructor<Dyn, Args>;

        fn try_emplace<Args>(
            self,
            constructor: Constructor<Dyn, Args>,
        ) -> Result<Self::Ptr, Self::Err<Args>> {
            self.get_mut()
                .try_emplace(constructor)
                .map(|ptr| unsafe { Pin::new_unchecked(ptr) })
        }
    }
}
pub use dyn_init::*;

// =========== Examples ===========
mod example {
    pub use super::*;

    pub trait Async {
        type Item;

        async fn foo(&mut self, arg: String) -> Self::Item;
    }

    pub trait DynAsync {
        type Item;

        fn foo(
            &mut self,
            arg: String,
        ) -> PinConstructor<dyn Future<Output = Self::Item>, (VoidPtr, String)>;
    }

    impl<T: Async + Sized> DynAsync for T {
        type Item = T::Item;

        fn foo(
            &mut self,
            arg: String,
        ) -> PinConstructor<dyn Future<Output = Self::Item>, (VoidPtr, String)>
        where
            Self: Sized,
        {
            unsafe {
                Constructor::new(
                    return_type_layout(&<Self as Async>::foo),
                    (NonNull::from(self).cast(), arg),
                    |slot, (this, arg)| {
                        let fun = <Self as Async>::foo;
                        let slot = return_type_cast_ptr(&fun, slot);
                        #[allow(unused_unsafe)]
                        unsafe {
                            let out = fun(this.cast().as_mut(), arg);
                            slot.write(out);
                            let ptr = slot.as_ptr() as *mut dyn Future<Output = T::Item>;
                            NonNull::new_unchecked(ptr)
                        }
                    },
                )
            }
            .pinned()
        }
    }
}
use example::*;

async fn dynamic_dispatch<Item: Eq + std::fmt::Debug>(
    imp: &mut dyn DynAsync<Item = Item>,
    arg: String,
) -> Item {
    let mut stack = std::pin::pin!([0u8; 16]);

    // compile fail:
    //
    // let a = imp.foo(arg.clone()).buffered(stack.as_mut());
    // let b = imp.foo(arg.clone()).buffered(stack.as_mut());

    let a = match imp.foo(arg.clone()).try_buffered(stack.as_mut()) {
        Ok(fut) => fut.await,
        Err(c) => c.boxed().await,
    };
    let b = match imp.foo(arg).try_buffered(stack.as_mut()) {
        Ok(fut) => fut.await,
        Err(c) => c.boxed().await,
    };
    assert_eq!(a, b);
    a
}

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    test_return_type_layout();

    struct AppendYay;
    impl Async for AppendYay {
        type Item = String;
        async fn foo(&mut self, arg: String) -> Self::Item {
            arg + ", yay!"
        }
    }
    struct PrintYay;
    impl Async for PrintYay {
        type Item = ();
        async fn foo(&mut self, arg: String) -> Self::Item {
            println!("{arg}, yay!");
        }
    }
    struct CheckYay<'a>(&'a str);
    impl<'a> Async for CheckYay<'a> {
        type Item = &'a str;
        async fn foo(&mut self, arg: String) -> Self::Item {
            assert_eq!(arg + ", yay!", self.0);
            self.0
        }
    }

    dynamic_dispatch(&mut PrintYay, "foo".to_owned()).await;

    let item = dynamic_dispatch(&mut AppendYay, "foo".to_owned()).await;
    let item = dynamic_dispatch(&mut CheckYay(&item), "foo".to_owned()).await;
    assert_eq!(item, "foo, yay!");
}
