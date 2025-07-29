/* This file is @generated for testing purpose */
fn test<'this, 'arg1, 'arg2, 'dynify>(
    &'this self,
    arg1: &'arg1 str,
    arg2: Context<'arg2>,
)
where
    'this: 'dynify,
    'arg1: 'dynify,
    'arg2: 'dynify,
    Self: 'dynify,
{}
