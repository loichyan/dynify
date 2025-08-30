/* This file is @generated for testing purpose */
trait Trait {
    async fn test(&self, arg: &str);
}
#[allow(async_fn_in_trait)]
#[allow(clippy::type_complexity)]
trait DynTrait {
    fn test<'this, 'arg, 'dynify>(
        &'this self,
        arg: &'arg str,
    ) -> ::dynify::r#priv::Fn<
        (::dynify::r#priv::RefSelf, &'arg str),
        dyn 'dynify + ::core::future::Future<Output = ()>,
    >
    where
        'this: 'dynify,
        'arg: 'dynify,
        Self: 'dynify;
}
#[allow(clippy::type_complexity)]
impl<TraitImplementor: Trait> DynTrait for TraitImplementor {
    fn test<'this, 'arg, 'dynify>(
        &'this self,
        arg: &'arg str,
    ) -> ::dynify::r#priv::Fn<
        (::dynify::r#priv::RefSelf, &'arg str),
        dyn 'dynify + ::core::future::Future<Output = ()>,
    >
    where
        'this: 'dynify,
        'arg: 'dynify,
        Self: 'dynify,
    {
        ::dynify::__from_fn!([self] TraitImplementor::test, self, arg,)
    }
}
fn main() {}
