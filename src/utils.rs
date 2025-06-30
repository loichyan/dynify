use core::mem::ManuallyDrop;

/// Defines a macro with its internal rules hidden on rustdoc.
macro_rules! doc_macro {
    ($(#[$attr:meta])* macro $name:ident $documented:tt $real:tt) => {
        #[cfg(doc)] $(#[$attr])* macro_rules! $name $documented
        #[cfg(not(doc))] $(#[$attr])* macro_rules! $name $real
    };
}

/// Registers callbacks when exiting the current scope.
#[allow(non_camel_case_types)]
pub(crate) struct Defer<F: FnOnce()>(ManuallyDrop<F>);
impl<F: FnOnce()> Drop for Defer<F> {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::take(&mut self.0)() }
    }
}
pub(crate) fn defer<F: FnOnce()>(f: F) -> Defer<F> {
    Defer(ManuallyDrop::new(f))
}
