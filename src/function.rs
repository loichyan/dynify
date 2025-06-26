use crate::constructor::{Constructor, Slot};
use crate::receiver::Receiver;
use core::alloc::Layout;
use core::marker::PhantomData;
use core::ptr::NonNull;

pub struct Fn<Args, Ret: ?Sized> {
    layout: Layout,
    init: unsafe fn(Slot, Args) -> NonNull<Ret>,
    args: Args,
}
impl<Args, Ret: ?Sized> Fn<Args, Ret> {
    pub unsafe fn from_static<F>(
        _: &F,
        args: Args,
        init: unsafe fn(Slot, Args) -> NonNull<Ret>,
    ) -> Self
    where
        F: FunctionType<Args>,
    {
        Self {
            layout: Layout::new::<F::Ret>(),
            init,
            args,
        }
    }
    pub unsafe fn from_method<A, F>(
        _: &F,
        args: Args,
        init: unsafe fn(Slot, Args) -> NonNull<Ret>,
    ) -> Self
    where
        F: FunctionType<A>,
        Method<A, F::Ret>: FunctionType<Args>,
    {
        Self::from_static(&Method::<A, F::Ret>(PhantomData), args, init)
    }
}
unsafe impl<Args, Ret: ?Sized> Constructor for Fn<Args, Ret> {
    type Object = Ret;
    unsafe fn construct(self, slot: Slot) -> NonNull<Self::Object> {
        (self.init)(slot, self.args)
    }
    fn layout(&self) -> Layout {
        self.layout
    }
}

pub trait FunctionType<Args> {
    type Ret;
}
/// Wraps a function with its receiver type sealed.
pub struct Method<Args, Ret>(PhantomData<fn(Args) -> Ret>);
macro_rules! impl_function {
    ($a:ident $(,$i:ident)* -> $r:ident) => {
        impl<Fn, $a, $($i,)* $r> FunctionType<($a, $($i,)*)> for Fn
        where
            Fn: FnOnce($a, $($i,)*) -> $r,
        {
            type Ret = $r;
        }
        impl<$a, $($i,)* $r> FunctionType<(<$a as Receiver>::Sealed, $($i,)*)> for Method<($a, $($i,)*), $r>
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

#[test]
fn check_return_type_layout() {
    use std::any::Any;
    use std::convert::Infallible;

    pub const fn layout<A, F: FunctionType<A>>(_: &F) -> Layout {
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
