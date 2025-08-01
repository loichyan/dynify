/* This file is @generated for testing purpose */
#[allow(async_fn_in_trait)]
trait Trait<'life1, 'life2>
where
    'life2: 'life1,
    Self: 'static + Send,
{
    const KST: usize;
    type Type: 'static;
    async fn method(&self);
    async fn fun(this: &Self);
}
#[allow(async_fn_in_trait)]
#[allow(clippy::type_complexity)]
trait DynTrait<'life1, 'life2>
where
    'life2: 'life1,
    Self: 'static + Send,
{
    const KST: usize;
    type Type: 'static;
    fn method<'this, 'dynify>(
        &'this self,
    ) -> ::dynify::r#priv::Fn<
        (::dynify::r#priv::RefSelf,),
        dyn 'dynify + ::core::future::Future<Output = ()>,
    >
    where
        'this: 'dynify,
        'life1: 'dynify,
        'life2: 'dynify,
        Self: 'dynify;
    async fn fun(this: &Self);
}
#[allow(clippy::type_complexity)]
impl<'life1, 'life2, TraitImplementor: Trait<'life1, 'life2>> DynTrait<'life1, 'life2>
for TraitImplementor
where
    'life2: 'life1,
    Self: 'static + Send,
{
    const KST: usize = TraitImplementor::KST;
    type Type = TraitImplementor::Type;
    fn method<'this, 'dynify>(
        &'this self,
    ) -> ::dynify::r#priv::Fn<
        (::dynify::r#priv::RefSelf,),
        dyn 'dynify + ::core::future::Future<Output = ()>,
    >
    where
        'this: 'dynify,
        'life1: 'dynify,
        'life2: 'dynify,
        Self: 'dynify,
    {
        ::dynify::from_fn!(TraitImplementor::method, self,)
    }
    async fn fun(this: &Self) {
        TraitImplementor::fun(this).await
    }
}
