use rcell::*;

#[test]
fn std_type() {
    let rcell = RCell::new("foobar");
    assert!(rcell.retained());
}

#[test]
fn own_type() {
    struct MyType(&'static str);
    let rcell = RCell::new(MyType("foobar"));
    assert!(rcell.retained());
}
