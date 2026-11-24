#![cfg(windows)]

use std::{
    env,
    ffi::{OsStr, OsString},
    io,
    path::Path,
};
use windows_sys::Win32::UI::WindowsAndMessaging as win;
use winreg::{
    RegKey,
    enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_EXPAND_SZ, REG_SZ},
    types::{FromRegValue, ToRegValue},
};

/// 将绝对目录追加到 Windows 用户 PATH；已存在时返回 `false`。
pub fn add_to_path(dir: &Path) -> io::Result<bool> {
    if !dir.is_absolute() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "需要绝对目录"));
    }

    let (environment, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey_with_flags("Environment", KEY_QUERY_VALUE | KEY_SET_VALUE)?;
    let added = append_to_path(&environment, dir)?;
    if added {
        notify_environment_changed();
    }
    Ok(added)
}

fn append_to_path(environment: &RegKey, dir: &Path) -> io::Result<bool> {
    let contains = |path: &OsStr| {
        env::split_paths(path).any(|entry| {
            entry
                .iter()
                .map(OsStr::to_ascii_lowercase)
                .eq(dir.iter().map(OsStr::to_ascii_lowercase))
        })
    };
    if env::var_os("PATH").is_some_and(|path| contains(&path)) {
        return Ok(false);
    }

    let (mut path, kind) = match environment.get_raw_value("Path") {
        Ok(value) if matches!(value.vtype, REG_SZ | REG_EXPAND_SZ) => {
            (OsString::from_reg_value(&value)?, value.vtype)
        }
        Ok(_) => return Err(io::Error::other("用户 PATH 不是字符串，未修改")),
        Err(err) if err.kind() == io::ErrorKind::NotFound => (OsString::new(), REG_SZ),
        Err(err) => return Err(err),
    };
    if contains(&path) {
        return Ok(false);
    }

    // 保留原有条目和 REG_EXPAND_SZ 类型，不展开其中的变量。
    if !path.is_empty() && !path.as_encoded_bytes().ends_with(b";") {
        path.push(";");
    }
    path.push(env::join_paths([dir]).map_err(io::Error::other)?);
    let mut value = path.to_reg_value();
    value.vtype = kind;
    environment.set_raw_value("Path", &value)?;
    Ok(true)
}

fn notify_environment_changed() {
    let name: Vec<_> = "Environment\0".encode_utf16().collect();
    // SAFETY: 同步发送消息，UTF-16 字符串在调用期间有效。
    unsafe {
        win::SendMessageTimeoutW(
            win::HWND_BROADCAST,
            win::WM_SETTINGCHANGE,
            0,
            name.as_ptr() as isize,
            win::SMTO_ABORTIFHUNG,
            1000,
            std::ptr::null_mut(),
        );
    }
}

#[cfg(test)]
#[path = "../tests/add-path.rs"]
mod tests;
