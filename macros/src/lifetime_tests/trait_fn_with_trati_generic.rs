/* This file is @generated for testing purpose */
fn test<'this, 'arg, 'dynify>(this: &'this Self, arg: &'arg str)
where
    'this: 'dynify,
    'arg: 'dynify,
    Arg: 'dynify,
{}
