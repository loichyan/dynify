fn test<'this, 'dynify>(&'this self, arg: fn(&str) -> &str)
where
    'this: 'dynify,
    Self: 'dynify,
{}
