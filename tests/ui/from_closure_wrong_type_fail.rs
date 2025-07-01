fn main() {
    dynify::from_closure(|slot| slot.write(123i32) as &mut dynify::Opaque<u32>);
}
