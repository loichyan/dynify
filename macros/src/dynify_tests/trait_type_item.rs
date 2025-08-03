/* This file is @generated for testing purpose */
#[allow(async_fn_in_trait)]
trait Trait {
    type Type: 'static;
}
#[allow(async_fn_in_trait)]
#[allow(clippy::type_complexity)]
trait DynTrait {
    type Type: 'static;
}
#[allow(clippy::type_complexity)]
impl<TraitImplementor: Trait> DynTrait for TraitImplementor {
    type Type = TraitImplementor::Type;
}
fn main() {}
