#![cfg_attr(not(test), no_std)]
#![allow(unsafe_op_in_unsafe_fn)]
#![deny(clippy::unsound_collection_transmute)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod closure;
mod constructor;
mod container;
mod function;
mod receiver;

pub use self::closure::from_closure;
pub use self::constructor::{Construct, Dynify, PinConstruct, PinDynify, Slot};
#[cfg(feature = "alloc")]
pub use self::container::Boxed;
pub use self::container::{Buffered, Emplace, OutOfCapacity, PinEmplace};

/// NON-PUBLIC API
#[doc(hidden)]
pub mod r#priv {
    pub use crate::function::{from_bare_fn, from_method, Fn};
    pub use crate::receiver::{ArcSelf, BoxSelf, RcSelf, Receiver, RefMutSelf, RefSelf};

    pub type PinRefSelf<'a> = crate::receiver::Pin<RefSelf<'a>>;
    pub type PinRefMutSelf<'a> = crate::receiver::Pin<RefMutSelf<'a>>;
    pub type PinBoxSelf = crate::receiver::Pin<BoxSelf>;
    pub type PinRcSelf = crate::receiver::Pin<RcSelf>;
    pub type PinArcSelf = crate::receiver::Pin<ArcSelf>;
}

type VoidPtr = core::ptr::NonNull<Void>;
enum Void {}

// =========== Examples ===========
#[cfg(test)]
#[pollster::test]
pub async fn test_example() {
    use std::future::Future;
    use std::mem::MaybeUninit;

    pub trait Async {
        type Item;
        const NAME: &'static str;

        async fn foo(&mut self, arg: String) -> Self::Item;
    }

    #[allow(clippy::type_complexity)]
    pub trait DynAsync {
        type Item;

        fn name(&self) -> &'static str;

        fn foo<'a>(
            &'a mut self,
            arg: String,
        ) -> Dynify<Fn!(&'a mut Self, String => dyn 'a + Future<Output = Self::Item>)>;
    }

    impl<T: Async + Sized> DynAsync for T {
        type Item = T::Item;

        fn name(&self) -> &'static str {
            T::NAME
        }

        fn foo<'a>(
            &'a mut self,
            arg: String,
        ) -> Dynify<Fn!(&'a mut Self, String => dyn 'a + Future<Output = Self::Item>)> {
            from_fn!(<Self as Async>::foo, self, arg)
        }
    }

    #[allow(dead_code)]
    struct AppendYay;
    impl Async for AppendYay {
        type Item = String;
        const NAME: &'static str = "AppendYay";
        async fn foo(&mut self, arg: String) -> Self::Item {
            arg + ", yay!"
        }
    }
    struct PrintYay;
    impl Async for PrintYay {
        type Item = ();
        const NAME: &'static str = "PrintYay";
        async fn foo(&mut self, arg: String) -> Self::Item {
            println!("{arg}, yay!");
        }
    }
    struct CheckYay<'a>(&'a str);
    impl<'a> Async for CheckYay<'a> {
        type Item = &'a str;
        const NAME: &'static str = "CheckYay";
        async fn foo(&mut self, arg: String) -> Self::Item {
            let a = [0u8; 64];
            async { assert_eq!(arg + ", yay!", self.0) }.await;
            _ = a;
            self.0
        }
    }

    async fn dynamic_dispatch<Item: Eq + std::fmt::Debug>(
        imp: &mut dyn DynAsync<Item = Item>,
        arg: String,
    ) -> Item {
        let mut stack = [MaybeUninit::<u8>::uninit(); 64];
        let mut heap = Vec::<MaybeUninit<u8>>::new();

        // compile fail:
        //
        // let a = imp.foo(arg.clone());
        // let b = imp.foo(arg.clone());
        // a.boxed();
        // b.boxed();

        // compile fail:
        //
        // let a = imp.foo(arg.clone()).buffered(stack.as_mut());
        // let b = imp.foo(arg.clone()).buffered(stack.as_mut());

        let name = imp.name();
        println!(">>> {name}, layout={:?}", imp.foo(arg.clone()).layout());
        let a = imp
            .foo(arg.clone())
            .try_init(&mut stack)
            .inspect(|_| println!(">>> stack allocated {name}"))
            .inspect_err(|_| println!(">>> heap allocated {name}"))
            .unwrap_or_else(|(c, _)| c.init(&mut heap))
            .await;
        let b = imp.foo(arg.clone()).init2(&mut stack, &mut heap).await;
        assert_eq!(a, b);
        a
    }

    dynamic_dispatch(&mut PrintYay, "foo".to_owned()).await;
    let item = dynamic_dispatch(&mut AppendYay, "foo".to_owned()).await;
    let item = dynamic_dispatch(&mut CheckYay(&item), "foo".to_owned()).await;
    assert_eq!(item, "foo, yay!");
}
