/* This file is @generated for testing purpose */
#[allow(async_fn_in_trait)]
trait Trait {
    async fn test(this: &Self, arg: &str);
}
#[allow(async_fn_in_trait)]
#[allow(clippy::type_complexity)]
trait DynTrait {
    async fn test(this: &Self, arg: &str);
}
#[allow(clippy::type_complexity)]
impl<TraitImplementor: Trait> DynTrait for TraitImplementor {
    async fn test(this: &Self, arg: &str) {
        TraitImplementor::test(this, arg).await
    }
}
