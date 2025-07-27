fn test<'arg, 'dynify, T>(arg: &'arg str)
where
    'arg: 'dynify,
    T: 'dynify,
{}
