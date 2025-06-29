/// Compile tests.
#[test]
#[cfg_attr(any(miri, coverage), ignore)] // compile tests are meaningless for coverage
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*_fail.rs");
    t.pass("tests/ui/*_pass.rs");
}
