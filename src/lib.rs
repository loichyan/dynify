#![doc = include_str!("lib.md") ]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(test), no_std)]
#![allow(unsafe_op_in_unsafe_fn)]
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

#[doc = include_str!("dynify.md") ]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
#[cfg(feature = "macros")]
pub use dynify_macros::dynify;

#[doc(inline)]
#[cfg(feature = "alloc")]
pub use self::container::Boxed;
#[doc(inline)]
pub use self::{
    closure::from_closure,
    constructor::{Construct, Dynify, Opaque, PinConstruct, PinDynify, Slot},
    container::{Buffered, Emplace, OutOfCapacity, PinEmplace},
};

/// NON-PUBLIC API
#[doc(hidden)]
pub mod r#priv {
    pub use crate::function::{from_bare_fn, from_method, Fn};
    #[cfg(feature = "alloc")]
    pub use crate::receiver::{ArcSelf, BoxSelf, RcSelf};
    pub use crate::receiver::{Receiver, RefMutSelf, RefSelf};

    pub type PinRefSelf<'a> = crate::receiver::Pin<RefSelf<'a>>;
    pub type PinRefMutSelf<'a> = crate::receiver::Pin<RefMutSelf<'a>>;
    #[cfg(feature = "alloc")]
    pub type PinBoxSelf = crate::receiver::Pin<BoxSelf>;
    #[cfg(feature = "alloc")]
    pub type PinRcSelf = crate::receiver::Pin<RcSelf>;
    #[cfg(feature = "alloc")]
    pub type PinArcSelf = crate::receiver::Pin<ArcSelf>;
}

#[doc = include_str!("../README.md")]
pub const _: () = {};
