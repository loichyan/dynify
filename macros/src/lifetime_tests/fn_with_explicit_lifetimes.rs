fn test<'this1, 'arg20, 'this0, 'arg1, 'arg21, 'dynify>(
    self: MySelf<'this0, 'this1>,
    arg1: &'arg1 str,
    arg2: (&'arg20 str, &'arg21 str),
)
where
    'arg20: 'dynify,
    'this1: 'dynify,
    'this0: 'dynify,
    'arg1: 'dynify,
    'arg21: 'dynify,
    Self: 'dynify,
{}
