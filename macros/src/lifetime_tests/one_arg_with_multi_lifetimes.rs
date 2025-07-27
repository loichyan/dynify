fn test<'this0, 'this1, 'arg0, 'arg1, 'dynify>(
    self: MySelf<'this0, 'this1>,
    arg: (&'arg0 str, &'arg1 str),
)
where
    'this0: 'dynify,
    'this1: 'dynify,
    'arg0: 'dynify,
    'arg1: 'dynify,
    Self: 'dynify,
{}
