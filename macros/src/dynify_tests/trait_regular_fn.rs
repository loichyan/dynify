/* This file is @generated for testing purpose */
#[allow(async_fn_in_trait)]
trait Trait {
    fn test(arg: &Self, arg: &str);
}
#[allow(async_fn_in_trait)]
#[allow(clippy::type_complexity)]
trait DynTrait {
    fn test(arg: &Self, arg: &str);
}
#[allow(clippy::type_complexity)]
impl<TraitImplementor: Trait> DynTrait for TraitImplementor {
    fn test(arg: &Self, arg: &str) {
        TraitImplementor::test(arg, arg)
    }
}
