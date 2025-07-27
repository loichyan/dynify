fn test<'arg, 'dynify>(arg: &'arg str)
where
    'arg: 'dynify,
{}
