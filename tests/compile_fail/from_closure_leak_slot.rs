fn main() {
    let mut slot_leaked = None;
    let _ = dynify::from_closure::<(), (), _>(|slot| {
        slot_leaked = Some(slot);
        unreachable!();
    });
}
