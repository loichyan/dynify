#![allow(mismatched_lifetime_syntaxes)]

use std::any::Any;
use std::pin::{pin, Pin};

use super::*;
use crate::receiver::*;
use crate::utils::{randstr, DropCounter, StrFut};
use crate::{Dynify, PinDynify};

#[test]
fn return_type_layout_ok() {
    use std::any::Any;
    use std::convert::Infallible;

    pub const fn layout<A, F: Function<A>>(_: &F) -> Layout {
        Layout::new::<F::Ret>()
    }

    fn f1(_: usize, _: usize) -> usize {
        todo!()
    }
    fn f2(_: &str) -> &str {
        todo!()
    }
    fn f3(_: String, _: Vec<u8>, _: &dyn Any) -> Box<dyn Any> {
        todo!()
    }
    fn f4(_: usize, _: usize) -> Infallible {
        todo!()
    }

    assert_eq!(layout(&f1), Layout::new::<usize>());
    assert_eq!(layout(&f2), Layout::new::<&str>());
    assert_eq!(layout(&f3), Layout::new::<Box<dyn Any>>());
    assert_eq!(layout(&f4), Layout::new::<Infallible>());
}

#[pollster::test]
async fn from_bare_fn_ok() {
    thread_local! {
        static DATA: String = randstr(8..64);
    }
    let init: Fn!(=> StrFut) = from_fn!(|| async { DATA.with(<_>::clone) });
    assert_eq!(init.pin_boxed().await, DATA.with(<_>::clone));
}

#[test]
fn from_method_ok() {
    struct Test<'a>(&'a str);
    impl Test<'_> {
        fn test(&self) -> Fn!(&Self => dyn Any) {
            from_fn!(|this: &Self| this.0.to_owned(), self)
        }
    }
    let data = randstr(8..64);
    let test = Test(&data);
    let init = test.test();
    assert_eq!(init.boxed().downcast_ref::<String>(), Some(&data));
}

#[test]
#[allow(clippy::drop_non_drop)]
fn from_fn_drop_ok() {
    let init: Fn!(=> dyn Any) = from_fn!(|| DropCounter);
    // Nothing happens if a function pointer gets dropped.
    drop(init);
    assert_eq!(DropCounter::count(), 0);

    let arg = DropCounter;
    let init: Fn!(DropCounter => dyn Any) = from_fn!(|arg| { std::mem::forget(arg) }, arg);
    // Nothing happens if `DropCounter` gets forgotten.
    let _ = init.boxed();
    assert_eq!(DropCounter::count(), 0);

    let arg = DropCounter;
    let init: Fn!(DropCounter => dyn Any) = from_fn!(|_| DropCounter, arg);
    // Count increments if `DropCounter` gets dropped.
    drop(init);
    assert_eq!(DropCounter::count(), 1);
}

struct Test;
#[allow(clippy::boxed_local)]
impl Test {
    fn ref_fn(&self) {}
    fn ref_mut_fn(&mut self) {}
    fn box_fn(self: Box<Self>) {}
    fn rc_fn(self: std::rc::Rc<Self>) {}
    fn arc_fn(self: std::sync::Arc<Self>) {}
    fn pin_fn(self: Pin<&mut Self>) {}
    fn pin_box_fn(self: Pin<Box<Self>>) {}
}

#[rustfmt::skip]
    impl Test {
        fn as_ref(&self)                      -> Fn!(&Self => ())          { from_fn!(Self::ref_fn, self) }
        fn as_ref_mut(&mut self)              -> Fn!(&mut Self => ())      { from_fn!(Self::ref_mut_fn, self) }
        fn as_box(self: Box<Self>)            -> Fn!(Box<Self> => ())      { from_fn!(Self::box_fn, self) }
        fn as_rc(self: std::rc::Rc<Self>)     -> Fn!(Rc<Self> => ())       { from_fn!(Self::rc_fn, self) }
        fn as_arc(self: std::sync::Arc<Self>) -> Fn!(Arc<Self> => ())      { from_fn!(Self::arc_fn, self) }
        fn as_pin(self: Pin<&mut Self>)       -> Fn!(Pin<&mut Self> => ()) { from_fn!(Self::pin_fn, self) }
        fn as_pin_box(self: Pin<Box<Self>>)   -> Fn!(Pin<Box<Self>> => ()) { from_fn!(Self::pin_box_fn, self) }
    }
#[test]
fn receiver_matching() {
    let _: Fn<(RefSelf,), ()> = Test.as_ref();
    let _: Fn<(RefMutSelf,), ()> = Test.as_ref_mut();
    let _: Fn<(BoxSelf,), ()> = Box::new(Test).as_box();
    let _: Fn<(RcSelf,), ()> = std::rc::Rc::new(Test).as_rc();
    let _: Fn<(ArcSelf,), ()> = std::sync::Arc::new(Test).as_arc();
    let _: Fn<(crate::receiver::Pin<RefMutSelf>,), ()> = pin!(Test).as_pin();
    let _: Fn<(crate::receiver::Pin<BoxSelf>,), ()> = Box::pin(Test).as_pin_box();
}

#[rustfmt::skip]
    impl Test {
        fn as_bare_ref(this: &Self)     -> Fn!(&Test => ())                 { from_fn!(Self::ref_fn, this) }
        fn as_bare_box(this: Box<Self>) -> Fn!(std::boxed::Box<Self> => ()) { from_fn!(Self::box_fn, this) }
    }
#[test]
fn receiver_match_fallback() {
    let _: Fn<(&Test,), ()> = Test::as_bare_ref(&Test);
    let _: Fn<(Box<Test>,), ()> = Test::as_bare_box(Box::new(Test));
}
