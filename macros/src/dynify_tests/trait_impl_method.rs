/* This file is @generated for testing purpose */
#[allow(async_fn_in_trait)]
trait Trait {
    fn test(&self, arg: &str) -> impl std::any::Any;
}
#[allow(async_fn_in_trait)]
#[allow(clippy::type_complexity)]
trait DynTrait {
    fn test<'this, 'arg, 'dynify>(
        &'this self,
        arg: &'arg str,
    ) -> ::dynify::r#priv::Fn<
        (::dynify::r#priv::RefSelf, &'arg str),
        dyn 'dynify + std::any::Any,
    >
    where
        'this: 'dynify,
        'arg: 'dynify,
        Self: 'dynify;
}
#[allow(clippy::type_complexity)]
impl<TraitImplementor: Trait> DynTrait for TraitImplementor {
    fn test<'this, 'arg, 'dynify>(
        &'this self,
        arg: &'arg str,
    ) -> ::dynify::r#priv::Fn<
        (::dynify::r#priv::RefSelf, &'arg str),
        dyn 'dynify + std::any::Any,
    >
    where
        'this: 'dynify,
        'arg: 'dynify,
        Self: 'dynify,
    {
        ::dynify::__from_fn!([self] TraitImplementor::test, self, arg,)
    }
}
fn main() {}
