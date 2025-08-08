#[dynify::dynify]
trait Trait {
    async fn test(&self, (arg): String, NewType(inner): NewType);
}

fn main() {}
