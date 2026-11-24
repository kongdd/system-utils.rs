use super::*;

#[test]
fn parses_targets() {
    assert!(matches!(target(["--stdout".into()].into_iter()), Ok(Target::Stdout)));
    assert!(matches!(
        target(["--output".into(), "shot.png".into()].into_iter()),
        Ok(Target::File(path)) if path == "shot.png"
    ));
}
