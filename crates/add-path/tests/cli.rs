use super::*;

#[test]
fn expands_tilde_to_home() -> std::io::Result<()> {
    let home = env::home_dir().expect("缺少用户目录");
    let bin = home.join(".local").join("bin");
    assert_eq!(expand_home(Path::new("~/.local/bin"))?, bin);
    assert_eq!(expand_home(Path::new(r"~\.local\bin"))?, bin);
    Ok(())
}

#[test]
fn force_creates_missing_parent_directories() -> std::io::Result<()> {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("parent").join("bin");

    prepare_dir(&dir, false)?;
    assert!(!dir.exists());

    prepare_dir(&dir, true)?;
    prepare_dir(&dir, true)?;
    assert!(dir.is_dir());
    Ok(())
}
