/* This file is @generated for testing purpose */
#[allow(async_fn_in_trait)]
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
        ::dynify::from_fn!(TraitImplementor::test, self,)
    }
}
