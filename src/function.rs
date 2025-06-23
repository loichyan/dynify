use core::alloc::Layout;

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
impl_function!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P -> R);

pub const fn layout_of_return_type<A, F: Function<A>>(_: &F) -> Layout {
    Layout::new::<F::Output>()
}

#[test]
fn check_return_type_layout() {
    use std::any::Any;
    use std::convert::Infallible;

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

    assert_eq!(Layout::new::<usize>(), layout_of_return_type(&f1));
    assert_eq!(Layout::new::<&str>(), layout_of_return_type(&f2));
    assert_eq!(Layout::new::<Box<dyn Any>>(), layout_of_return_type(&f3));
    assert_eq!(Layout::new::<Infallible>(), layout_of_return_type(&f4));
    println!("test_return_type_layout pass");
}
