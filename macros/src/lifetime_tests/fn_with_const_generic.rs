fn test<'arg, 'dynify, const N: usize>(arg: &'arg str)
where
    'arg: 'dynify,
{}
