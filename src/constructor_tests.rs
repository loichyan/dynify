use std::mem;

use rstest::rstest;

use crate::utils::{boxed_slice, randarr, DebugAny, DropCounter, OpqAny};
use crate::{from_closure, Dynify, Emplace, PinDynify};

struct UnsafePinnedContainer<C>(C);
unsafe impl<T, D> Emplace<T> for UnsafePinnedContainer<D>
where
    T: ?Sized,
    D: Emplace<T>,
{
    type Ptr = D::Ptr;
    type Err = D::Err;
    fn emplace<C>(self, constructor: C) -> Result<Self::Ptr, Self::Err>
    where
        C: crate::Construct<Object = T>,
    {
        self.0.emplace(constructor)
    }
}
// SAFETY: For testing purpose only, use it carefully.
unsafe impl<T, C> crate::PinEmplace<T> for UnsafePinnedContainer<C>
where
    T: ?Sized,
    C: Emplace<T>,
{
}
impl<C: ?Sized> UnsafePinnedContainer<&'_ mut C> {
    fn as_mut(&mut self) -> UnsafePinnedContainer<&mut C> {
        UnsafePinnedContainer(self.0)
    }
}

#[rstest]
#[case(0, randarr::<0>())]
#[case(1, randarr::<0>())]
#[case(4, randarr::<4>())]
#[case(7, randarr::<5>())]
fn init_ok<const N: usize>(#[case] stk_size: usize, #[case] data: [u8; N]) {
    let mut stk = boxed_slice(stk_size);

    let init = from_closure(|slot| slot.write(data) as &mut OpqAny);
    let out = init.init(&mut *stk);
    assert_eq!(out.downcast_ref::<[u8; N]>(), Some(&data));
    drop(out);

    let mut stk = UnsafePinnedContainer(&mut *stk);

    let init = from_closure(|slot| slot.write(data) as &mut OpqAny);
    let out = init.pin_init(stk.as_mut());
    assert_eq!(out.downcast_ref::<[u8; N]>(), Some(&data));
}

#[rstest]
#[case(0, 4, randarr::<4>())]
#[case(4, 5, randarr::<5>())]
#[case(0, 8, randarr::<6>())]
#[case(6, 9, randarr::<7>())]
fn init2_ok<const N: usize>(
    #[case] stk1_size: usize,
    #[case] stk2_size: usize,
    #[case] data: [u8; N],
) {
    let mut stk1 = boxed_slice(stk1_size);
    let mut stk2 = boxed_slice(stk2_size);

    let mut init = from_closure(|slot| slot.write(data) as &mut OpqAny);
    (init, _) = init.try_init(&mut *stk1).unwrap_err();
    let out = init.init2(&mut *stk1, &mut *stk2);
    assert_eq!(out.downcast_ref::<[u8; N]>(), Some(&data));
    drop(out);

    let mut stk1 = UnsafePinnedContainer(&mut *stk1);
    let mut stk2 = UnsafePinnedContainer(&mut *stk2);

    let mut init = from_closure(|slot| slot.write(data) as &mut OpqAny);
    (init, _) = init.try_pin_init(stk1.as_mut()).unwrap_err();
    let out = init.pin_init2(stk1.as_mut(), stk2.as_mut());
    assert_eq!(out.downcast_ref::<[u8; N]>(), Some(&data));
}

#[rstest]
#[case(0, 0, randarr::<7>())]
#[case(6, 0, randarr::<7>())]
#[case(0, 8, randarr::<9>())]
#[case(7, 8, randarr::<9>())]
#[should_panic = "failed to initialize"]
fn panic_on_init_fail(
    #[case] stk1_size: usize,
    #[case] stk2_size: usize,
    #[case] val: impl DebugAny,
) {
    let mut stk1 = boxed_slice(stk1_size);
    let mut stk2 = boxed_slice(stk2_size);

    let init = from_closure(|slot| slot.write(val));
    if stk2_size == 0 {
        init.init(&mut *stk1);
    } else {
        init.init2(&mut *stk1, &mut *stk2);
    }
}

#[rstest]
#[case(0, 0, randarr::<7>())]
#[case(6, 0, randarr::<7>())]
#[case(0, 8, randarr::<9>())]
#[case(7, 8, randarr::<9>())]
#[should_panic = "failed to initialize"]
fn panic_on_pin_init_fail(
    #[case] stk1_size: usize,
    #[case] stk2_size: usize,
    #[case] val: impl DebugAny,
) {
    let mut stk1 = boxed_slice(stk1_size);
    let mut stk1 = UnsafePinnedContainer(&mut *stk1);
    let mut stk2 = boxed_slice(stk2_size);
    let mut stk2 = UnsafePinnedContainer(&mut *stk2);

    let init = from_closure(|slot| slot.write(val));
    if stk2_size == 0 {
        init.init(stk1.as_mut());
    } else {
        init.init2(stk1.as_mut(), stk2.as_mut());
    }
}

#[test]
fn drop_boxed() {
    assert_eq!(DropCounter::count(), 0);

    let init = from_closure(|slot| slot.write(DropCounter) as &mut OpqAny);
    drop(init.boxed());
    assert_eq!(DropCounter::count(), 1);

    let init = from_closure(|slot| slot.write(DropCounter) as &mut OpqAny);
    drop(init.pin_boxed());
    assert_eq!(DropCounter::count(), 2);
}

#[rstest]
#[case(randarr::<4>())]
#[case(randarr::<8>())]
#[case(randarr::<16>())]
#[case(randarr::<32>())]
fn fallible_constructor(#[case] val: impl DebugAny) {
    let mut stack = randarr::<8>();
    let mut heap = vec![0u8; 16];

    let val_size = mem::size_of_val(&val);
    let mut init = Some(from_closure(|slot| slot.write(val)));
    let init = &mut init;

    if val_size <= stack.len() {
        assert!(init.try_init(&mut stack).is_ok());
    } else {
        assert!(init.try_init(&mut stack).is_err());
        assert!(init.try_init2(&mut stack, &mut heap).is_ok());
    }
}
