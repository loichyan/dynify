#[dynify_macros::dynify]
#[allow(async_fn_in_trait)]
pub trait MyAsync {
    async fn foo(&self) -> usize;
}
