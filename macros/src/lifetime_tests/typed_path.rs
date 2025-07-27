fn test<'arg, 'dynify>(arg: Context<'arg>)
where
    'arg: 'dynify,
{}
