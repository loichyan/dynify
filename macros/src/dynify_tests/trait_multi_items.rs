/* This file is @generated for testing purpose */
#[allow(async_fn_in_trait)]
trait Trait {
    const KST1: usize;
    const KST2: bool;
    type Type1: 'static;
    type Type2: core::future::Future<Output = ()>;
    async fn method1(&self) -> Vec<u8>;
    fn method2(&self);
    async fn fun1(this: &Self) -> String;
    fn fun2(this: &Self) -> impl core::future::Future<Output = String>;
}
#[allow(async_fn_in_trait)]
#[allow(clippy::type_complexity)]
trait DynTrait {
    const KST1: usize;
    const KST2: bool;
    type Type1: 'static;
    type Type2: core::future::Future<Output = ()>;
    fn method1<'this, 'dynify>(
        &'this self,
    ) -> ::dynify::r#priv::Fn<
        (::dynify::r#priv::RefSelf,),
        dyn 'dynify + ::core::future::Future<Output = Vec<u8>>,
    >
    where
        'this: 'dynify,
        Self: 'dynify;
    fn method2(&self);
    async fn fun1(this: &Self) -> String;
    fn fun2(this: &Self) -> impl core::future::Future<Output = String>;
}
#[allow(clippy::type_complexity)]
impl<TraitImplementor: Trait> DynTrait for TraitImplementor {
    const KST1: usize = TraitImplementor::KST1;
    const KST2: bool = TraitImplementor::KST2;
    type Type1 = TraitImplementor::Type1;
    type Type2 = TraitImplementor::Type2;
    fn method1<'this, 'dynify>(
        &'this self,
    ) -> ::dynify::r#priv::Fn<
        (::dynify::r#priv::RefSelf,),
        dyn 'dynify + ::core::future::Future<Output = Vec<u8>>,
    >
    where
        'this: 'dynify,
        Self: 'dynify,
    {
        ::dynify::from_fn!(TraitImplementor::method1, self,)
    }
    fn method2(&self) {
        TraitImplementor::method2(self)
    }
    async fn fun1(this: &Self) -> String {
        TraitImplementor::fun1(this).await
    }
    fn fun2(this: &Self) -> impl core::future::Future<Output = String> {
        TraitImplementor::fun2(this)
    }
}
fn main() {}
