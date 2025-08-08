#[cfg_attr(miri, ignore)] // compile tests are meaningless for Miri
#[test]
fn compile_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/*.rs");
    t.pass("macros/src/dynify_tests/*.rs");

    if rustversion::cfg!(stable(1.80)) {
        t.compile_fail("tests/compile_fail/*.rs");
    } else {
        // Skip UI tests that depend on rustc
        t.compile_fail("tests/compile_fail/dynify_*.rs");
    }
}
