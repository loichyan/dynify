fn main() {
    let _: dynify::Fn!(=> u32) = dynify::from_fn!(|| 123i32);
}
