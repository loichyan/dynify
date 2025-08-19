#[dynify::dynify]
fn test1() {}

#[dynify::dynify]
fn test2() -> FakeImpl {}

#[dynify::dynify]
opaque_trait!();

fn main() {}
