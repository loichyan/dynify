fn test<'this, 'dynify>(&'this self)
where
    'this: 'dynify,
    Self: 'dynify,
{}
