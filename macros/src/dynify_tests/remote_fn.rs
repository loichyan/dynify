/* This file is @generated for testing purpose */
fn dyn_test_remote_fn<'_arg1, 'dynify>(
    _arg1: &'_arg1 str,
) -> ::dynify::r#priv::Fn<
    (&'_arg1 str,),
    dyn 'dynify + ::core::future::Future<Output = usize>,
>
where
    '_arg1: 'dynify,
{
    ::dynify::__from_fn!([] dynify::r#priv::test_remote_fn, _arg1,)
}
fn main() {}
