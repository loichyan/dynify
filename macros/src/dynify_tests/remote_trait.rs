/* This file is @generated for testing purpose */
#[allow(async_fn_in_trait)]
#[allow(clippy::type_complexity)]
trait DynTestRemoteTrait {
    fn test<'this, 'arg, 'dynify>(
        &'this self,
        arg: &'arg str,
    ) -> ::dynify::r#priv::Fn<
        (::dynify::r#priv::RefSelf, &'arg str),
        dyn 'dynify + ::core::future::Future<Output = usize>,
    >
    where
        'this: 'dynify,
        'arg: 'dynify,
        Self: 'dynify;
}
#[allow(clippy::type_complexity)]
impl<TestRemoteTraitImplementor: dynify::r#priv::TestRemoteTrait> DynTestRemoteTrait
for TestRemoteTraitImplementor {
    fn test<'this, 'arg, 'dynify>(
        &'this self,
        arg: &'arg str,
    ) -> ::dynify::r#priv::Fn<
        (::dynify::r#priv::RefSelf, &'arg str),
        dyn 'dynify + ::core::future::Future<Output = usize>,
    >
    where
        'this: 'dynify,
        'arg: 'dynify,
        Self: 'dynify,
    {
        ::dynify::__from_fn!([self] TestRemoteTraitImplementor::test, self, arg,)
    }
}
fn main() {}
