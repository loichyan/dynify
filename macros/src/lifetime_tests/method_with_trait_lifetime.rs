fn test<'this, 'arg, 'dynify>(&'this self, arg: &'arg str)
where
    'this: 'dynify,
    'arg: 'dynify,
    'Life: 'dynify,
    Self: 'dynify,
{}
