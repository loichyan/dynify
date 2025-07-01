use core::alloc::Layout;
use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::constructor::{Construct, Opaque, PinConstruct, Slot};
use crate::receiver::Receiver;

/// A constructor for the return type of functions.
pub struct Fn<Args, Ret: ?Sized> {
    layout: Layout,
    init: unsafe fn(Slot, Args) -> &mut Opaque<Ret>,
    args: Args,
}

unsafe impl<Args, Ret: ?Sized> PinConstruct for Fn<Args, Ret> {
    type Object = Ret;
    unsafe fn construct(self, slot: Slot) -> NonNull<Self::Object> {
        let ptr = (self.init)(slot, self.args);
        NonNull::from(ptr.as_mut())
    }
    fn layout(&self) -> Layout {
        self.layout
    }
}
unsafe impl<Args, Ret: ?Sized> Construct for Fn<Args, Ret> {}

/// A helper struct to display friendly errors.
///
/// For the emitted errors, see `tests/ui/from_fn_with_closure_fail.stderr`.
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
#[inline(always)]
pub unsafe fn from_bare_fn<F, Args, Ret>(
    _: fn(MustNotBeClosure) -> F,
    args: Args,
    init: unsafe fn(Slot, Args) -> &mut Opaque<Ret>,
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
#[inline(always)]
pub unsafe fn from_method<F, Args, Ret>(
    _: fn(MustNotBeClosure) -> F,
    args: Args,
    init: unsafe fn(Slot, F::SealedArgs) -> &mut Opaque<Ret>,
) -> Fn<F::SealedArgs, Ret>
where
    F: Method<Args>,
    Ret: ?Sized,
{
    pub struct MethodAsBareFn<Args, F>(PhantomData<(fn(Args), F)>);
    impl<Args, F> Function<F::SealedArgs> for MethodAsBareFn<Args, F>
    where
        F: Method<Args>,
    {
        type Ret = F::Ret;
    }
    let args = F::seal_args(args);
    from_bare_fn(|_| MethodAsBareFn::<Args, F>(PhantomData), args, init)
}

/// A blanked trait implemented for arbitrary functions.
pub trait Function<Args> {
    type Ret;
}
/// Wraps a function with its receiver type sealed.
pub trait Method<Args>: Function<Args> {
    type SealedArgs;
    fn seal_args(args: Args) -> Self::SealedArgs;
}
macro_rules! impl_function {
    (-> $R:ident) => {
        impl<Fn: FnOnce() -> $R, $R> Function<()> for Fn {
            type Ret = $R;
        }
    };
    ($A:ident $(,$Args:ident)* -> $R:ident) => {
        impl<Fn, $A, $($Args,)* $R> Function<($A, $($Args,)*)> for Fn
        where
            Fn: FnOnce($A, $($Args,)*) -> $R,
        {
            type Ret = $R;
        }
        impl<Fn, $A, $($Args,)* $R> Method<($A, $($Args,)*)> for Fn
        where
            $A: Receiver,
            Fn: FnOnce($A, $($Args,)*) -> $R,
        {
            type SealedArgs = (<$A as Receiver>::Sealed, $($Args,)*);
            #[allow(non_snake_case)]
            #[inline(always)]
            fn seal_args(($A, $($Args,)*): ($A, $($Args,)*)) -> Self::SealedArgs {
                (Receiver::seal($A), $($Args,)*)
            }
        }
        impl_function!($($Args),* -> $R);
    };
}
impl_function!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P -> R); // 16 arguments

doc_macro! {
    /// Creates a constructor for static functions.
    ///
    /// It accepts as its parameters the target function followed by all the
    /// arguments required to invoke that function, returning a constructor for
    /// the return type of the function. The type of returned constructors can
    /// be obtained with [`Fn`].
    ///
    /// The provided function must be a static item which can be resolved at
    /// compile-time; therefore, closures are not supported. For methods, the
    /// second parameter must be `self`; otherwise, the returned constructor
    /// falls back to a bare function constructor.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dynify::{from_fn, Fn};
    /// # use std::future::Future;
    /// async fn read_string(path: &str) -> String { String::new() }
    /// let path = "/tmp/file";
    /// let _: Fn!(_ => dyn Future<Output = String>) = from_fn!(read_string, path);
    /// ```
    ///
    /// [`Fn`]: crate::Fn
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
            $crate::r#priv::from_method(
                |_| $f,
                ($self, $($args,)*),
                |slot, (this, $($args,)*)| {
                    let this = $crate::r#priv::Receiver::unseal(this);
                    let ret = ($f)(this, $($args,)*);
                    let ptr = slot.cast().write(ret);
                    ptr as &mut $crate::Opaque::<_>
                },
            )
        }
    };
    ([$($_:ident)?] $f:expr, $($args:ident,)*) => {
        // SAFETY: See the comment above.
        unsafe {
            $crate::r#priv::from_bare_fn(
                |_| $f,
                ($($args,)*),
                |slot, ($($args,)*)| {
                    let ret = ($f)($($args,)*);
                    let ptr = slot.cast().write(ret);
                    ptr as &mut $crate::Opaque::<_>
                },
            )
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
    /// # use dynify::{from_fn, Fn};
    /// # use std::future::Future;
    /// async fn fetch_something(uri: &str) -> String {
    ///     String::from("** mysterious text **")
    /// }
    /// fn dyn_fetch_something(uri: &str) -> Fn!(&str => dyn '_ + Future<Output = String>) {
    ///     from_fn!(fetch_something, uri)
    /// }
    /// ```
    ///
    /// [`from_fn`]: crate::from_fn
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "function_tests.rs"]
mod tests;
