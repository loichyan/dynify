# 🦕 dynify

[![crates.io](https://img.shields.io/crates/v/dynify)](https://crates.io/crates/dynify)
[![docs.rs](https://img.shields.io/docsrs/dynify)](https://docs.rs/dynify)
[![msrv](https://img.shields.io/crates/msrv/dynify)](https://crates.io/crates/dynify)
[![build status](https://img.shields.io/github/actions/workflow/status/loichyan/dynify/cicd.yml)](https://github.com/loichyan/dynify/actions)
[![codecov](https://img.shields.io/codecov/c/gh/loichyan/dynify)](https://codecov.io/gh/loichyan/dynify)

Add dyn compatible variant to your async trait with dynify!

## ✨ Overview

dynify implements partial features of the experimental
[in-place initialization proposal](https://github.com/rust-lang/lang-team/issues/336)
in stable Rust, along with a set of safe APIs for creating in-place constructors
to initialize trait objects. Here’s a quick example of how to use dynify:

```rust
use dynify::Dynify;
use std::future::Future;
use std::mem::MaybeUninit;

// `AsyncRead` is dyn incompatible :(
// With dynify, we can create a dyn compatible variant for `AsyncRead` in one line :)
#[dynify::dynify]
trait AsyncRead { // By default, another trait prefixed with `Dyn` is generated.
    async fn read_to_string(&mut self) -> String;
}

// Now we can use dynamic dispatched `AsyncRead`!
async fn dynamic_dispatch(reader: &mut dyn DynAsyncRead) {
    let mut stack = [MaybeUninit::<u8>::uninit(); 16];
    let mut heap = Vec::<MaybeUninit<u8>>::new();
    // Initialize trait objects on the stack if not too large, otherwise on the heap.
    let fut = reader.read_to_string().init2(&mut stack, &mut heap);
    let content = fut.await;
    // ...
}
```

For a more detailed explanation, check out the
[API documentation](https://docs.rs/dynify).

## 🔍 Comparisons with other similar projects

### vs pin-init

[pin-init](https://crates.io/crates/pin-init) has been around for a while and
provides safe methods for creating in-place constructors for `struct`s. It also
has an
[experimental branch](https://github.com/Rust-for-Linux/pin-init/tree/dev/experimental/dyn)
that enables the generation of dyn compatible variants for `async fn`s. The key
difference is that pin-init relies on some nightly features, while dynify is
built with stable Rust. Moreover, as their names suggest, pin-init is focused on
the pinned initialization of structures, whereas dynify targets dyn
compatibility for functions. With its ongoing `#[dyn_init]` feature, pin-init
can be considered as a superset of dynify.

### vs async-trait

[async-trait](https://crates.io/crates/async-trait) is another widely used crate
for dynamic dispatch on AFIT (Async Fn In Trait). The main advantage of dynify
is its ability to allocate trait objects on the stack, making it more suitable
for limited environments. In contrast, async-trait requires heap allocation to
store trait objects, as it essentially transforms `async fn` into
`Box<dyn Future>`.

## ♥️ Special thanks

- [Rust-for-Linux/pin-init](https://github.com/Rust-for-Linux/pin-init) for its
  brilliant design on creating constructors for `async fn`s, which serves as the
  foundation of dynify.
- [In-place initialization proposal](https://hackmd.io/@aliceryhl/BJutRcPblx)
  for its excellent design on initializer traits, which is incorporated into
  several trait designs of dynify.
- [zjp-CN/dyn-afit](https://github.com/zjp-CN/dyn-afit) for the comprehensive
  comparisons of community solutions for dynamic dispatch on AFIT, which greatly
  inspired dynify.

## ⚖️ License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
