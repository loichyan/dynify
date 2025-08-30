/* This file is @generated for testing purpose */
trait Trait {}
#[allow(async_fn_in_trait)]
#[allow(clippy::type_complexity)]
trait DynTrait {}
#[allow(clippy::type_complexity)]
impl<TraitImplementor: Trait> DynTrait for TraitImplementor {}
fn main() {}
