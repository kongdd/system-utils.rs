use super::*;

#[test]
fn appends_without_overwriting_or_duplicating() -> io::Result<()> {
    let sandbox = tempfile::tempdir()?;
    let bin = sandbox.path().join("bin");

    // 使用临时注册表项，不写真实的 HKCU\Environment。
    let user = RegKey::predef(HKEY_CURRENT_USER);
    let name = format!(
        "Software\\add-path-test-{}",
        sandbox.path().file_name().unwrap().display()
    );
    let (key, _) = user.create_subkey(&name)?;
    let result = (|| -> io::Result<_> {
        let original = r"%LOCALAPPDATA%\Tools;C:\Other";
        let mut value = original.to_reg_value();
        value.vtype = REG_EXPAND_SZ;
        key.set_raw_value("Path", &value)?;
        let process_path = std::env::var_os("PATH").unwrap_or_default();
        if let Some(entry) = std::env::split_paths(&process_path).find(|dir| dir.is_absolute()) {
            let skipped = !append_to_path(&key, &entry)?;
            if !skipped {
                return Err(io::Error::other("不应重复追加进程 PATH 中的目录"));
            }
        }
        let added = append_to_path(&key, &bin)?;
        let added_again = append_to_path(&key, &bin.join(""))?;
        let path: OsString = key.get_value("Path")?;
        let value = key.get_raw_value("Path")?;
        Ok((added, added_again, path, value.vtype))
    })();
    drop(key);
    user.delete_subkey_all(name)?;

    let (added, added_again, path, kind) = result?;
    assert!(added && !added_again);
    assert_eq!(kind, REG_EXPAND_SZ);
    let paths: Vec<_> = std::env::split_paths(&path).collect();
    assert_eq!(
        paths,
        [r"%LOCALAPPDATA%\Tools".into(), r"C:\Other".into(), bin]
    );
    Ok(())
}
