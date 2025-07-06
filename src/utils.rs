#![allow(dead_code)]

use core::mem::ManuallyDrop;

pub enum Void {}
pub(crate) type VoidPtr = core::ptr::NonNull<Void>;

/// Defines a macro with its internal rules hidden on rustdoc.
macro_rules! doc_macro {
    ($(#[$attr:meta])* macro $name:ident $documented:tt $real:tt) => {
        #[cfg(doc)] $(#[$attr])* macro_rules! $name $documented
        #[cfg(not(doc))] $(#[$attr])* macro_rules! $name $real
    };
}

pub(crate) struct Defer<F: FnOnce()>(ManuallyDrop<F>);
impl<F: FnOnce()> Drop for Defer<F> {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::take(&mut self.0)() }
    }
}
/// Registers callbacks when exiting the current scope.
pub(crate) fn defer<F: FnOnce()>(f: F) -> Defer<F> {
    Defer(ManuallyDrop::new(f))
}

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod test_utils {
    use core::mem::MaybeUninit;
    use std::any::Any;
    use std::cell::Cell;
    use std::fmt;
    use std::future::Future;
    use std::ops::RangeBounds;

    pub(crate) trait DebugAny: Any + fmt::Debug {}
    impl<T: Any + fmt::Debug> DebugAny for T {}

    pub(crate) type StrFut<'a> = dyn 'a + Future<Output = String>;
    pub(crate) type OpqStrFut<'a> = crate::Opaque<dyn 'a + Future<Output = String>>;
    pub(crate) type OpqAny<'a> = crate::Opaque<dyn 'a + Any>;

    /// A thread-local counter that increments when it gets dropped.
    pub(crate) struct DropCounter;
    thread_local! {
        static COUNT: Cell<usize> = const { Cell::new(0) };
    }
    impl Drop for DropCounter {
        fn drop(&mut self) {
            COUNT.set(COUNT.get() + 1);
        }
    }
    impl DropCounter {
        pub fn count() -> usize {
            COUNT.get()
        }
    }

    pub(crate) fn randarr<const N: usize>() -> [u8; N] {
        let mut arr = [0; N];
        arr.fill_with(|| fastrand::alphanumeric() as u32 as u8);
        arr
    }

    pub(crate) fn newstk<const N: usize>() -> [MaybeUninit<u8>; N] {
        [MaybeUninit::uninit(); N]
    }

    pub(crate) fn randstr(len: impl RangeBounds<usize>) -> String {
        std::iter::repeat_with(fastrand::alphanumeric)
            .take(fastrand::usize(len))
            .collect()
    }

    pub(crate) fn newheap(len: usize) -> Vec<MaybeUninit<u8>> {
        vec![MaybeUninit::uninit(); len]
    }

    pub(crate) fn newheap_fixed(len: usize) -> Box<[MaybeUninit<u8>]> {
        newheap(len).into_boxed_slice()
    }
}
#[cfg(test)]
pub(crate) use test_utils::*;
