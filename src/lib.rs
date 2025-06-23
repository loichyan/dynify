#![cfg_attr(not(test), no_std)]
#![allow(async_fn_in_trait)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::missing_safety_doc)]
#![deny(clippy::unsound_collection_transmute)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod constructor;
mod container;
mod function;

pub use self::constructor::{Constructor, PinConstructor, Slot};
#[cfg(feature = "alloc")]
pub use self::container::Boxed;
pub use self::container::{Buffered, Container, PinContainer};

/// NON-PUBLIC API
#[doc(hidden)]
pub mod r#priv {
    pub use crate::constructor::{new_constructor, Receiver, ReceiverMut};
    pub use crate::function::layout_of_return_type;
}

// =========== Examples ===========
#[cfg(test)]
#[pollster::test]
pub async fn test_example() {
    use crate::r#priv::*;
    use std::future::Future;
    use std::mem::MaybeUninit;
    use std::ptr::NonNull;

    pub trait Async {
        type Item;
        const NAME: &'static str;

        async fn foo(&mut self, arg: String) -> Self::Item;
    }

    pub trait DynAsync {
        type Item;

        fn name(&self) -> &'static str;

        fn foo<'a>(
            &'a mut self,
            arg: String,
        ) -> PinConstructor<dyn 'a + Future<Output = Self::Item>, (ReceiverMut<'a>, String)>;
    }

    impl<T: Async + Sized> DynAsync for T {
        type Item = T::Item;

        fn name(&self) -> &'static str {
            T::NAME
        }

        fn foo<'a>(
            &'a mut self,
            arg: String,
        ) -> PinConstructor<dyn 'a + Future<Output = Self::Item>, (ReceiverMut<'a>, String)>
        where
            Self: Sized,
        {
            unsafe {
                new_constructor(
                    layout_of_return_type(&<Self as Async>::foo),
                    (ReceiverMut::new(self), arg),
                    |slot, (this, arg)| {
                        let out = <Self as Async>::foo(this.get(), arg);
                        let ptr = slot.write(out);
                        ptr as NonNull<dyn Future<Output = T::Item>>
                    },
                )
                .pinned()
            }
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
        let mut stack = std::pin::pin!([MaybeUninit::<u8>::uninit(); 64]);
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
            .try_init(stack.as_mut())
            .inspect(|_| println!(">>> stack allocated {name}"))
            .inspect_err(|_| println!(">>> heap allocated {name}"))
            .unwrap_or_else(|c| c.init(&mut heap))
            .await;
        let b = imp.foo(arg.clone()).init2(stack.as_mut(), &mut heap).await;
        assert_eq!(a, b);
        a
    }

    dynamic_dispatch(&mut PrintYay, "foo".to_owned()).await;
    let item = dynamic_dispatch(&mut AppendYay, "foo".to_owned()).await;
    let item = dynamic_dispatch(&mut CheckYay(&item), "foo".to_owned()).await;
    assert_eq!(item, "foo, yay!");
}
