use std::any::Any;
use std::marker::PhantomPinned;
use std::pin::pin;

use rstest::rstest;
#[cfg(feature = "smallvec")]
use smallvec::SmallVec;

use super::*;
use crate::utils::*;
use crate::{from_closure, Dynify};

trait DebugEmplace: Emplace<dyn Any, Err = Self::__Err> {
    type __Err: std::fmt::Debug;
}
impl<C> DebugEmplace for C
where
    C: Emplace<dyn Any>,
    C::Err: std::fmt::Debug,
{
    type __Err = C::Err;
}

#[rstest]
#[case(&mut MaybeUninit::<[u8; 12]>::uninit())]
#[case(&mut [MaybeUninit::new(0u8); 12])]
#[case(&mut [MaybeUninit::new(0u8); 12] as &mut [MaybeUninit<u8>])]
fn fix_sized_containers<C>(#[case] c: &mut C)
where
    C: ?Sized,
    for<'a> &'a mut C: DebugEmplace,
{
    let inp = randarr::<8>();
    let init = from_closure(|slot| slot.write(inp) as &mut OpqAny);
    let out = c.emplace(init).unwrap();
    assert_eq!(out.downcast_ref::<[u8; 8]>(), Some(&inp), "init ok");
    drop(out);

    let inp = randarr::<14>();
    let init = from_closure(|slot| slot.write(inp) as &mut OpqAny);
    assert!(c.emplace(init).is_err(), "init err");
}

#[rstest]
#[case(Boxed)]
#[case(&mut Vec::<MaybeUninit<u8>>::new())]
#[cfg_attr(feature = "smallvec", case(&mut SmallVec::<[MaybeUninit<u8>; 0]>::new()) )]
#[cfg_attr(feature = "smallvec", case(&mut SmallVec::<[MaybeUninit<u8>; 12]>::new()) )]
fn allocated_containers(#[case] c: impl DebugEmplace) {
    let inp = randarr::<16>();
    let init = from_closure(|slot| slot.write(inp) as &mut OpqAny);
    let out = c.emplace(init).unwrap();
    assert_eq!(out.downcast_ref::<[u8; 16]>(), Some(&inp));
}

#[rstest]
#[case(Boxed)]
#[case(&mut [MaybeUninit::new(0u8); 64])]
#[case(&mut [MaybeUninit::uninit(); 64] as &mut [MaybeUninit<u8>])]
#[case(&mut Vec::<MaybeUninit<u8>>::new())]
#[cfg_attr(feature = "smallvec", case(&mut SmallVec::<[MaybeUninit<u8>; 0]>::new()) )]
#[cfg_attr(feature = "smallvec", case(&mut SmallVec::<[MaybeUninit<u8>; 12]>::new()) )]
fn init_object_of_random_layout(#[case] c: impl DebugEmplace) {
    macro_rules! select_layout {
        ($rand:ident, $($align:literal),+) => {$(
            if $rand == $align {
                #[repr(align($align))]
                struct Test<T>(T);
                let inp = randarr::<16>();
                let init = from_closure(|slot| slot.write(Test(inp)) as &mut OpqAny);
                let out = c.emplace(init).unwrap();
                let out = out.downcast_ref::<Test<[u8; 16]>>().unwrap();
                assert_eq!(&out.0, &inp);
                return;
            }
        )*};
    }

    let rand = 1 << fastrand::usize(..6);
    select_layout!(rand, 1, 2, 4, 8, 16, 32);
    unreachable!();
}

#[rstest]
#[case(Boxed)]
#[case(&mut Vec::<MaybeUninit<u8>>::new())]
#[case(&mut [] as &mut [MaybeUninit<u8>])]
#[case(&mut [] as &mut [MaybeUninit<u8>; 0])]
#[cfg_attr(feature = "smallvec", case(&mut SmallVec::<[MaybeUninit<u8>; 0]>::new()) )]
#[cfg_attr(feature = "smallvec", case(&mut SmallVec::<[MaybeUninit<u8>; 12]>::new()) )]
fn never_fail_on_zst(#[case] c: impl DebugEmplace) {
    #[repr(align(4096))]
    struct Zst;

    let init = from_closure(|slot| slot.write(Zst) as &mut OpqAny);
    let out = c.emplace(init).unwrap();
    let out = out.downcast_ref::<Zst>().unwrap();
    assert!(std::ptr::from_ref(out).is_aligned());
}

#[rstest]
#[case(&mut newstk::<24>())]
#[case(&mut newstk::<24>() as &mut [MaybeUninit<u8>])]
#[case(&mut Vec::<MaybeUninit<u8>>::new())]
#[cfg_attr(feature = "smallvec", case(&mut SmallVec::<[MaybeUninit<u8>; 0]>::new()) )]
#[cfg_attr(feature = "smallvec", case(&mut SmallVec::<[MaybeUninit<u8>; 12]>::new()) )]
fn drop_buffered<'a>(#[case] c: impl 'a + DebugEmplace<Ptr = Buffered<'a, dyn Any>>) {
    let init = from_closure(|slot| slot.write(DropCounter) as &mut OpqAny);
    let out = c.emplace(init).unwrap();
    assert_eq!(DropCounter::count(), 0);
    drop(out);
    assert_eq!(DropCounter::count(), 1);
}

#[test]
fn unpin_buffered() {
    let mut stack = newstk::<16>();
    let init = from_closure(|slot| slot.write(123));
    let val: Pin<&mut Buffered<usize>> = pin!(init.init(&mut stack));
    let _: &mut Buffered<usize> = Pin::into_inner(val);
}

#[test]
fn project_buffered() {
    let mut stack = newstk::<16>();
    let init = from_closure(|slot| slot.write(PhantomPinned));
    let mut buf: Pin<&mut Buffered<PhantomPinned>> = pin!(init.init(&mut stack));
    let _: Pin<&mut PhantomPinned> = buf.as_mut().project();
    let _: Pin<&PhantomPinned> = buf.as_ref().project_ref();
}

#[test]
fn project_pinned_buffered() {
    let mut stack = newstk::<16>();
    let init = from_closure(|slot| slot.write(123));
    let mut val: Pin<&mut Buffered<usize>> = pin!(init.init(&mut stack));
    let _: Pin<&mut usize> = val.as_mut().project();
    let _: Pin<&usize> = val.as_ref().project_ref();
}

#[pollster::test]
async fn buffered_future() {
    let mut stack = newstk::<16>();
    let inp = randstr(8..64);
    let init = from_closure(|slot| slot.write(async { inp.clone() }) as &mut OpqStrFut);
    let fut: Buffered<StrFut> = stack.emplace(init).unwrap();
    let out = fut.await;
    assert_eq!(out, inp);
}

#[test]
fn buffered_raw_ptr() {
    let mut stack = newstk::<16>();
    let stack_ptr = std::ptr::from_ref(&stack);
    let init = from_closure(|slot| slot.write([0u8; 4]));
    let val: Buffered<[u8; 4]> = stack.emplace(init).unwrap();
    let val_ptr = val.into_raw().as_ptr();
    assert_eq!(val_ptr as *const (), stack_ptr as *const ());
}

#[test]
fn default_pin_emplace() {
    let inp = randarr::<16>();
    let init = from_closure(|slot| slot.write(inp) as &mut OpqAny);
    let out = Boxed.pin_emplace(init).unwrap();
    assert_eq!(out.downcast_ref::<[u8; 16]>(), Some(&inp));
}

#[test]
#[should_panic = "just panic"]
fn clean_up_boxed_zst_on_panic() {
    let _ = from_closure::<(), (), _>(|_| panic!("just panic")).boxed();
}

#[test]
#[should_panic = "just panic"]
fn clean_up_boxed_on_panic() {
    let _ = from_closure::<usize, usize, _>(|_| panic!("just panic")).boxed();
}
