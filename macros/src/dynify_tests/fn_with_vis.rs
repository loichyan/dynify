/* This file is @generated for testing purpose */
#[allow(async_fn_in_trait)]
pub(crate) fn test() -> impl core::any::Any {
    todo!()
}
pub(crate) fn dyn_test<'dynify>() -> ::dynify::r#priv::Fn<
    (),
    dyn 'dynify + core::any::Any,
> {
    ::dynify::__from_fn!([] test,)
}
fn main() {}
