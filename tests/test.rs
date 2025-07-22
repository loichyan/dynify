#[dynify_macros::dynify]
#[allow(async_fn_in_trait)]
pub trait MyAsync<'x, T> {
    type A<'a, B: 'a, C: 'a>
    where
        Self: 'a;
    const HHH: usize;

    async fn foo(&self, s: &str, g: (&str, &str)) -> usize;
    async fn foo2<'a, 'g0, A>(&'a self, s: &str, g: (&'g0 str, A, &str)) -> usize;
    async fn foo3<'a, 'g, A>(&'a self, s: &str, g: (&'g str, A, &str)) -> usize;
    async fn foo4<'a, 'g, A>(this: &'a Self, s: &str, g: (&'g str, A, &str)) -> usize;
}
