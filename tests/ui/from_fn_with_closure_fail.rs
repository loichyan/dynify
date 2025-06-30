fn main() {
    let var = 123;
    dynify::from_fn!(move || var);
}
