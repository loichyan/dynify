fn test<'arg, 'dynify>(arg: Pin<&'arg mut str>)
where
    'arg: 'dynify,
{}
