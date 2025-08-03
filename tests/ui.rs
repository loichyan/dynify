#[cfg_attr(any(miri, coverage), ignore)] // compile tests are meaningless for coverage
#[test]
fn compile_tests() {
    let t = trybuild::TestCases::new();
    if rustversion::cfg!(stable(1.80)) {
        t.compile_fail("tests/compile_fail/*.rs"); //  pinned to avoid UI breakages
    }
    t.pass("tests/compile_pass/*.rs");
    t.pass("macros/src/dynify_tests/*.rs");
}
