fn test<'this, 'arg, 'dynify>(this: &'this Self, arg: &'arg str)
where
    'this: 'dynify,
    'arg: 'dynify,
    'Life: 'dynify,
{}
