fn main() {
    let mut slot_leaked = None;
    dynify::from_closure::<(), (), _>(|slot| {
        slot_leaked = Some(slot);
        unreachable!();
    });
}
