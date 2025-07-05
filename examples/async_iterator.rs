use std::future::Future;
use std::mem::MaybeUninit;

use dynify::{from_fn, Dynify, Fn};

trait Stream {
    type Item;
    async fn next(&mut self) -> Option<Self::Item>;
}

trait DynStream {
    type Item;
    fn next(&mut self) -> Fn!(&mut Self => dyn '_ + Future<Output = Option<Self::Item>>);
}
impl<T: Stream> DynStream for T {
    type Item = T::Item;
    fn next(&mut self) -> Fn!(&mut Self => dyn '_ + Future<Output = Option<Self::Item>>) {
        from_fn!(T::next, self)
    }
}

struct FromIter<I>(I);
impl<I> Stream for FromIter<I>
where
    I: Iterator,
{
    type Item = I::Item;
    async fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// Validates the input with dynamic dispatched streams!
async fn validate_data(data: &str, iter: &mut dyn DynStream<Item = char>) {
    let mut stack = [MaybeUninit::<u8>::uninit(); 16];
    let mut heap = Vec::<MaybeUninit<u8>>::new();
    let mut data_iter = data.chars();

    while let Some(ch) = iter.next().init2(&mut stack, &mut heap).await {
        let expected = data_iter.next().unwrap();
        println!("> yielded={}, expected={}", ch, expected);
        assert_eq!(ch, expected);
    }
    assert_eq!(data_iter.count(), 0);
}

#[pollster::main]
async fn main() {
    let data = std::iter::repeat_with(fastrand::alphanumeric)
        .take(fastrand::usize(16..32))
        .collect::<String>();
    println!("generated data: {}", data);

    let mut iter = FromIter(data.chars());
    validate_data(&data, &mut iter).await;
    println!("finished data validation");
}
