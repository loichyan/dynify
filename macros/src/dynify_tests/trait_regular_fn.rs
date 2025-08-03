/* This file is @generated for testing purpose */
#[allow(async_fn_in_trait)]
trait Trait {
    fn test(this: &Self, arg: &str);
}
#[allow(async_fn_in_trait)]
#[allow(clippy::type_complexity)]
trait DynTrait {
    fn test(this: &Self, arg: &str);
}
#[allow(clippy::type_complexity)]
impl<TraitImplementor: Trait> DynTrait for TraitImplementor {
    fn test(this: &Self, arg: &str) {
        TraitImplementor::test(this, arg)
    }
}
fn main() {}
