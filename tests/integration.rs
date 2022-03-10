use rcell::*;

#[test]
fn std_type() {
    struct TestTag;

    rcell!(&str, TestTag);

    let rcell = RCell::new("foobar");
    assert!(rcell.retained());
}

#[test]
fn own_type() {
    struct MyType(&'static str);

    rcell!(MyType);

    let rcell = RCell::new(MyType("foobar"));
    assert!(rcell.retained());
}
