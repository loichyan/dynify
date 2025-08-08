/* This file is @generated for testing purpose */
fn test<'this, 'arg, 'dynify>(&'this self, arg: &'arg str)
where
    'this: 'dynify,
    'arg: 'dynify,
    Self: 'dynify,
{}
