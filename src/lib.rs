#![doc = include_str!("lib.md") ]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(not(test), no_std)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(unknown_lints)]
#![deny(clippy::unsound_collection_transmute)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[macro_use]
mod utils;
mod closure;
mod constructor;
mod container;
mod function;
mod receiver;

pub use self::closure::from_closure;
pub use self::constructor::{Construct, Dynify, Opaque, PinConstruct, PinDynify, Slot};
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
