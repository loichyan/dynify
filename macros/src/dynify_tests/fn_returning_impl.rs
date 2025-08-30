/* This file is @generated for testing purpose */
fn test() -> impl core::any::Any {
    todo!()
}
fn dyn_test<'dynify>() -> ::dynify::r#priv::Fn<(), dyn 'dynify + core::any::Any> {
    ::dynify::__from_fn!([] test,)
}
fn main() {}
