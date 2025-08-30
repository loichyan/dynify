/* This file is @generated for testing purpose */
async fn test(_arg1: &str) -> String {
    todo!()
}
fn dyn_test<'_arg1, 'dynify>(
    _arg1: &'_arg1 str,
) -> ::dynify::r#priv::Fn<
    (&'_arg1 str,),
    dyn 'dynify + ::core::future::Future<Output = String>,
>
where
    '_arg1: 'dynify,
{
    ::dynify::__from_fn!([] test, _arg1,)
}
fn main() {}
