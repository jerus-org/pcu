//! Command-line behaviour, as a user sees it: `tests/cmd/*.trycmd`.

#[test]
fn cli_tests() {
    trycmd::TestCases::new()
        .default_bin_name("pcu")
        .case("tests/cmd/*.trycmd");
}
