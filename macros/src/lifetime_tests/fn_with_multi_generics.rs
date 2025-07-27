fn test<
    'this1,
    'arg20,
    'x,
    'y,
    'this0,
    'arg1,
    'arg21,
    'dynify,
    T,
    U,
    const N: usize,
    const M: bool,
>(self: MySelf<'this0, 'this1>, arg1: &'arg1 str, arg2: (&'arg20 str, &'arg21 str))
where
    'arg20: 'dynify,
    'this1: 'dynify,
    'this0: 'dynify,
    'arg1: 'dynify,
    'arg21: 'dynify,
    T: 'dynify,
    U: 'dynify,
    Self: 'dynify,
{}
