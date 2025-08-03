/* This file is @generated for testing purpose */
#[allow(async_fn_in_trait)]
trait Trait {
    const KST: usize;
}
#[allow(async_fn_in_trait)]
#[allow(clippy::type_complexity)]
trait DynTrait {
    const KST: usize;
}
#[allow(clippy::type_complexity)]
impl<TraitImplementor: Trait> DynTrait for TraitImplementor {
    const KST: usize = TraitImplementor::KST;
}
fn main() {}
