fn test<'x, 'this, 'dynify>(&'this self)
where
    'this: 'dynify,
    Self: 'dynify,
{}
