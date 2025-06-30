use core::alloc::Layout;
use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::constructor::{Construct, PinConstruct, Slot};
use crate::receiver::Receiver;

/// A constructor for the return type of functions.
pub struct Fn<Args, Ret: ?Sized> {
    layout: Layout,
    init: unsafe fn(Slot, Args) -> NonNull<Ret>,
    args: Args,
}

unsafe impl<Args, Ret: ?Sized> PinConstruct for Fn<Args, Ret> {
    type Object = Ret;
    unsafe fn construct(self, slot: Slot) -> NonNull<Self::Object> {
        (self.init)(slot, self.args)
    }
    fn layout(&self) -> Layout {
        self.layout
    }
}
unsafe impl<Args, Ret: ?Sized> Construct for Fn<Args, Ret> {}

impl<Args, Ret: ?Sized> From<Fn<Args, Ret>> for crate::constructor::Dynify<Fn<Args, Ret>> {
    fn from(value: Fn<Args, Ret>) -> Self {
        value.dynify()
    }
}
impl<Args, Ret: ?Sized> From<Fn<Args, Ret>> for crate::constructor::PinDynify<Fn<Args, Ret>> {
    fn from(value: Fn<Args, Ret>) -> Self {
        value.pin_dynify()
    }
}

/// A helper struct to display friendly errors.
pub struct MustNotBeClosure;

/// Creates a constructor for the return type of the specified function.
///
/// All arguments required for `F` should be packed into `args` as a tuple.
/// `args` is passed to `init` along with a slot to store the returned value
/// when the returned instance is ready to be constructed.
///
/// # Safety
///
/// `init` may not write data to the supplied slot of different layouts than the
/// return type of `F`.
pub const unsafe fn from_bare_fn<F, Args, Ret>(
    _: fn(MustNotBeClosure) -> F,
    args: Args,
    init: unsafe fn(Slot, Args) -> NonNull<Ret>,
) -> Fn<Args, Ret>
where
    F: Function<Args>,
    Ret: ?Sized,
{
    Fn {
        layout: Layout::new::<F::Ret>(),
        init,
        args,
    }
}

/// Creates a constructor for the return type of the specified method.
///
/// A method is a function of which receiver is sealed with [`Receiver::seal`].
///
/// # Safety
///
/// See [`from_bare_fn`].
pub const unsafe fn from_method<A, F, Args, Ret>(
    _: fn(MustNotBeClosure) -> F,
    args: Args,
    init: unsafe fn(Slot, Args) -> NonNull<Ret>,
) -> Fn<Args, Ret>
where
    F: Function<A>,
    Method<A, F::Ret>: Function<Args>,
    Ret: ?Sized,
{
    from_bare_fn(|_| Method::<A, F::Ret>(PhantomData), args, init)
}

/// A blanked trait implemented for arbitrary functions.
pub trait Function<Args> {
    type Ret;
}
/// Wraps a function with its receiver type sealed.
pub struct Method<Args, Ret>(PhantomData<fn(Args) -> Ret>);
impl<Fn, R> Function<()> for Fn
where
    Fn: FnOnce() -> R,
{
    type Ret = R;
}
macro_rules! impl_function {
    ($a:ident $(,$i:ident)* -> $r:ident) => {
        impl<Fn, $a, $($i,)* $r> Function<($a, $($i,)*)> for Fn
        where
            Fn: FnOnce($a, $($i,)*) -> $r,
        {
            type Ret = $r;
        }
        impl<$a, $($i,)* $r> Function<(<$a as Receiver>::Sealed, $($i,)*)> for Method<($a, $($i,)*), $r>
        where
            $a: Receiver,
        {
            type Ret = $r;
        }
    };
}
impl_function!(A                                              -> R);
impl_function!(A, B                                           -> R);
impl_function!(A, B, C                                        -> R);
impl_function!(A, B, C, D                                     -> R);
impl_function!(A, B, C, D, E                                  -> R);
impl_function!(A, B, C, D, E, F                               -> R);
impl_function!(A, B, C, D, E, F, G                            -> R);
impl_function!(A, B, C, D, E, F, G, H                         -> R);
impl_function!(A, B, C, D, E, F, G, H, I                      -> R);
impl_function!(A, B, C, D, E, F, G, H, I, J                   -> R);
impl_function!(A, B, C, D, E, F, G, H, I, J, K                -> R);
impl_function!(A, B, C, D, E, F, G, H, I, J, K, L             -> R);
impl_function!(A, B, C, D, E, F, G, H, I, J, K, L, M          -> R);
impl_function!(A, B, C, D, E, F, G, H, I, J, K, L, M, N       -> R);
impl_function!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O    -> R);
impl_function!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P -> R); // 16 arguments

doc_macro! {
    /// Creates [`Construct`] from static functions.
    ///
    /// It accepts as its parameters the target function followed by all the
    /// arguments required to invoke that function, returning a constructor for
    /// the return type of the function. The type of returned constructors can
    /// be obtained with [`Fn`](crate::Fn).
    ///
    /// The provided function must be a static item which can be resolved at
    /// compile-time; therefore, closures are not supported. For methods, the
    /// second parameter must be `self`; otherwise, the returned constructor
    /// falls back to a bare function constructor.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dynify::{Fn, from_fn};
    /// # use std::future::Future;
    /// async fn read_string(path: &str) -> String { String::new() }
    /// let path = "/tmp/file";
    /// let _: Fn!(_ => dyn Future<Output = String>) = from_fn!(read_string, path);
    /// ```
    #[macro_export]
    macro from_fn {
        ($f:expr, $self:ident $(,$args:ident)*) => {};
        ($f:expr $(,$args:ident)*) => {};
    } {
        ($f:expr, $self:ident $(,$args:ident)* $(,)?) => { $crate::__from_fn!([$self] $f, $self, $($args,)*) };
        ($f:expr $(,$args:ident)* $(,)?) => { $crate::__from_fn!([] $f, $($args,)*) };
    }
}
#[doc(hidden)]
#[macro_export]
macro_rules! __from_fn {
    ([self] $f:expr, $self:ident, $($args:ident,)*) => {
        // SAFETY: Pinned memory is not required to move the return value of a
        // function into the supplied slot. Therefore, it doesn't matter whether
        // the constructor is wrapped in `Dynify` or `PinDynify`.
        unsafe {
            ::core::convert::Into::into($crate::r#priv::from_method(
                |_| $f,
                ($crate::r#priv::Receiver::seal($self), $($args,)*),
                |slot, (this, $($args,)*)| {
                    let this = $crate::r#priv::Receiver::unseal(this);
                    let ret = ($f)(this, $($args,)*);
                    let ptr = slot.write(ret);
                    ptr as ::core::ptr::NonNull<_>
                },
            ))
        }
    };
    ([$($_:ident)?] $f:expr, $($args:ident,)*) => {
        // SAFETY: See the comment above.
        unsafe {
            ::core::convert::Into::into($crate::r#priv::from_bare_fn(
                |_| $f,
                ($($args,)*),
                |slot, ($($args,)*)| {
                    let ret = ($f)($($args,)*);
                    let ptr = slot.write(ret);
                    ptr as ::core::ptr::NonNull<_>
                },
            ))
        }
    };
}

doc_macro! {
    /// Determines the constructor type of a function.
    ///
    /// It accepts as its parameters a list of argument types of the target
    /// function followed by a fat-arrow (`=>`) and the return type of that
    /// function, returning the type of constructors created by [`from_fn`].
    ///
    /// For method types, which are functions with a receiver type such as
    /// `&Self`, `&mut Self` or `Box<Self>` as the first parameter, this macro
    /// automatically selects an appropriate sealed type to make the constructor
    /// *dyn compatible*.
    ///
    /// Note that the receiver type should not include the full path. Using
    /// types like `std::boxed::Box<Self>` will lead to an incorrect matching
    /// and cause the constructor type to be *dyn incompatible*. Nevertheless,
    /// it's not necessary to import these types beforehand.
    ///
    /// If none of the supported receiver types matches, it falls back to a bare
    /// function type.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dynify::{Fn, from_fn};
    /// # use std::future::Future;
    /// async fn fetch_something(uri: &str) -> String {
    ///     String::from("** mysterious text **")
    /// }
    /// fn dyn_fetch_something(uri: &str) -> Fn!(&str => dyn '_ + Future<Output = String>) {
    ///     from_fn!(fetch_something, uri)
    /// }
    /// ```
    #[macro_export]
    macro Fn {
        (_ => $ret:ty) => {};
        ($Self:ty $(,$args:ty)* => $ret:ty) => {};
        ($($args:ty),* => $ret:ty) => {};
    } {
        (_ => $ret:ty) => { $crate::r#priv::Fn<_, $ret> };
        ($($args:tt)*) => { $crate::__Fn!($($args)*) };
    }
}
#[doc(hidden)]
#[macro_export]
macro_rules! __Fn {
    (&$($lt:lifetime)? Self     $(,$args:ty)* => $ret:ty) => { $crate::r#priv::Fn<($crate::r#priv::RefSelf$(<$lt>)*, $($args,)*), $ret> };
    (&$($lt:lifetime)? mut Self $(,$args:ty)* => $ret:ty) => { $crate::r#priv::Fn<($crate::r#priv::RefMutSelf$(<$lt>)*, $($args,)*), $ret> };
    (Box<Self>                  $(,$args:ty)* => $ret:ty) => { $crate::r#priv::Fn<($crate::r#priv::BoxSelf, $($args,)*), $ret> };
    (Rc<Self>                   $(,$args:ty)* => $ret:ty) => { $crate::r#priv::Fn<($crate::r#priv::RcSelf, $($args,)*), $ret> };
    (Arc<Self>                  $(,$args:ty)* => $ret:ty) => { $crate::r#priv::Fn<($crate::r#priv::ArcSelf, $($args,)*), $ret> };

    (Pin<&$($lt:lifetime)? Self>     $(,$args:ty)* => $ret:ty) => { $crate::r#priv::Fn<($crate::r#priv::PinRefSelf$(<$lt>)*, $($args,)*), $ret> };
    (Pin<&$($lt:lifetime)? mut Self> $(,$args:ty)* => $ret:ty) => { $crate::r#priv::Fn<($crate::r#priv::PinRefMutSelf$(<$lt>)*, $($args,)*), $ret> };
    (Pin<Box<Self>>                  $(,$args:ty)* => $ret:ty) => { $crate::r#priv::Fn<($crate::r#priv::PinBoxSelf, $($args,)*), $ret> };
    (Pin<Rc<Self>>                   $(,$args:ty)* => $ret:ty) => { $crate::r#priv::Fn<($crate::r#priv::PinRcSelf, $($args,)*), $ret> };
    (Pin<Arc<Self>>                  $(,$args:ty)* => $ret:ty) => { $crate::r#priv::Fn<($crate::r#priv::PinArcSelf, $($args,)*), $ret> };

    ($($args:ty),* => $ret:ty) => { $crate::r#priv::Fn<($($args,)*), $ret> };
}

#[test]
fn check_return_type_layout() {
    use std::any::Any;
    use std::convert::Infallible;

    pub const fn layout<A, F: Function<A>>(_: &F) -> Layout {
        Layout::new::<F::Ret>()
    }

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

    assert_eq!(layout(&f1), Layout::new::<usize>());
    assert_eq!(layout(&f2), Layout::new::<&str>());
    assert_eq!(layout(&f3), Layout::new::<Box<dyn Any>>());
    assert_eq!(layout(&f4), Layout::new::<Infallible>());
}
