/// Compile tests.
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*_fail.rs");
    t.pass("tests/ui/*_pass.rs");
}
