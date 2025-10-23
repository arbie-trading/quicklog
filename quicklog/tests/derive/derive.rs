#[test]
fn derive() {
    let t = trybuild::TestCases::new();
    t.pass("tests/derive/derive_00.rs");
    t.pass("tests/derive/derive_01.rs");
    t.pass("tests/derive/derive_02.rs");
    t.pass("tests/derive/derive_03.rs");
    t.pass("tests/derive/derive_04.rs");
    t.pass("tests/derive/derive_05.rs");
    t.pass("tests/derive/derive_06_selective_generic.rs");
    t.pass("tests/derive/derive_07_multiple_generics.rs");
    t.pass("tests/derive/derive_08_nested_generics.rs");
    t.pass("tests/derive/derive_09_backward_compat.rs");
    t.pass("tests/derive/derive_10_unused_generics.rs");
}
