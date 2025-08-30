/* This file is @generated for testing purpose */
trait Trait {
    async fn test(&self);
}
#[allow(async_fn_in_trait)]
#[allow(clippy::type_complexity)]
trait MyDynTrait {
    fn test<'this, 'dynify>(
        &'this self,
    ) -> ::dynify::r#priv::Fn<
        (::dynify::r#priv::RefSelf,),
        dyn 'dynify + ::core::future::Future<Output = ()>,
    >
    where
        'this: 'dynify,
        Self: 'dynify;
}
#[allow(clippy::type_complexity)]
impl<TraitImplementor: Trait> MyDynTrait for TraitImplementor {
    fn test<'this, 'dynify>(
        &'this self,
    ) -> ::dynify::r#priv::Fn<
        (::dynify::r#priv::RefSelf,),
        dyn 'dynify + ::core::future::Future<Output = ()>,
    >
    where
        'this: 'dynify,
        Self: 'dynify,
    {
        ::dynify::__from_fn!([self] TraitImplementor::test, self,)
    }
}
fn main() {}
