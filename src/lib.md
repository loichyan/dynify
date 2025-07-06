Add dyn compatible variant to your async trait with 🦕 dynify!

## The problem

Currently, dynamic dispatch on AFIT (Async Fn In Trait) is not possible in Rust. For the following
code:

```rust,compile_fail
trait AsyncRead {
    async fn read_to_string(&mut self) -> String;
}

async fn dynamic_dispatch(reader: &dyn AsyncRead) {
    // ...
}
```

compiler will give you errors like this:

```text
error[E0038]: the trait `AsyncRead` cannot be made into an object
 --> src/lib.rs:12:36
  |
7 | async fn dynamic_dispatch(reader: &dyn AsyncRead) {
  |                                    ^^^^^^^^^^^^^ `AsyncRead` cannot be made into an object
  |
note: for a trait to be "object safe" it needs to allow building a vtable to allow the call to be resolvable dynamically
```

## The solution

dynify implements partial features of the experimental
[in-place initialization proposal](https://github.com/rust-lang/lang-team/issues/336), which makes
it possible to create a dyn compatible variant for `AsyncRead`:

```rust
# trait AsyncRead {
#     async fn read_to_string(&mut self) -> String;
# }
use dynify::{from_fn, Dynify, Fn};
use std::future::Future;
use std::mem::MaybeUninit;

trait DynAsyncRead {
    // `Fn!()` returns a dyn compatible type for the original async function.
    fn read_to_string(&mut self) -> Fn!(&mut Self => dyn '_ + Future<Output = String>);
}
impl<T: AsyncRead> DynAsyncRead for T {
    fn read_to_string(&mut self) -> Fn!(&mut Self => dyn '_ + Future<Output = String>) {
        // While `from_fn!()` lets you create a constructor of such type.
        from_fn!(T::read_to_string, self)
    }
}

// Now we can use dynamic dispatched `AsyncRead`!
async fn dynamic_dispatch(reader: &mut dyn DynAsyncRead) {
    // Prepare containers, we will see how they are used soon.
    let mut stack = [MaybeUninit::<u8>::uninit(); 16];
    let mut heap = Vec::<MaybeUninit<u8>>::new();

    // `read_to_string` returns a constructor, which can be considered as a
    // function pointer to `AsyncRead::read_to_string` along with all necessary
    // arguments to invoke it.
    let init = reader.read_to_string();
    // Therefore, we need to initialize the constructor to obtain the actual
    // `Future` before going on. `Dynify` offers a set of convenient methods to
    // do this. Since the size of the `Future` object is determined at runtime,
    // we can't know in advance what size containers can fit it. Here we use
    // `init2` to select a appropriate container for it. It accepts two
    // containers:
    let fut = init.init2(
        // the first one is allocated on the stack, allowing us to put the
        // object there to avoid relatively costly heap allocations.
        &mut stack,
        // the second one is allocated on the heap, serving as a fallback if the
        // size of the object exceeds the capacity of our stack.
        &mut heap,
    );
    // Finally, we get the `Future`. Now poll it to obtain the output!
    let content = fut.await;
    // ...
}
```

## Why not async-trait?

[async-trait](https://crates.io/crates/async-trait) is the most popular crate for the aforementioned
problem. However, it may not play well with limited environments such as kernels or embedded
systems, as it transforms every `async fn()` into `fn() -> Box<dyn Future>`, requiring heap
allocation. dynify doesn't have such limitation, since you can decide where to place trait objects.
Additionally, you can opt out of the `alloc` feature to completely turn off heap allocation.

Furthermore, dynify offers some unique features compared to async-trait. One of them, as shown in
the example below, is the ability to reuse buffers across different trait objects:

```rust
# use dynify::{from_fn, Dynify, Fn, PinDynify};
# use std::future::Future;
# use std::mem::MaybeUninit;
# use std::pin::Pin;
trait Stream {
    type Item;
    async fn next(&mut self) -> Option<Self::Item>;
}

trait DynStream {
    type Item;
    fn next(&mut self) -> Fn!(&mut Self => dyn '_ + Future<Output = Option<Self::Item>>);
    fn next_boxed(&mut self) -> Pin<Box<dyn '_ + Future<Output = Option<Self::Item>>>>;
}

async fn process_stream(stream: &mut dyn DynStream<Item = char>) {
    let mut stack = [MaybeUninit::<u8>::uninit(); 16];
    let mut heap = Vec::<MaybeUninit<u8>>::new();

    // With dynify, all items are stored in the same buffer.
    while let Some(item) = stream.next().init2(&mut stack, &mut heap).await {
        // ...
    }
    // While with async-trait, every item is stored in a unique `Box`.
    while let Some(item) = stream.next_boxed().await {
        // ...
    }
}
```

Nevertheless, the differences can be rather trivial in many cases. If you don't have these concerns,
it's better to go with the battle tested async-trait.

## Features

- **alloc**: Enable container implementations for types that require heap allocation such as `Box`
  and `Vec`.
- **smallvec**: Enable container implementations for `SmallVec`, a drop-in replacement for
  `[u8; N] + Vec<u8>`.
