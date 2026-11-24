use std::{env, path::Path, process::ExitCode};

const USAGE: &str = "用法：add-path [-f] <绝对目录>";

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let (dir, force) = match (args.next(), args.next()) {
        (Some(arg), None) if arg == "-h" || arg == "--help" => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        (Some(flag), Some(dir)) if flag == "-f" && args.next().is_none() => (dir, true),
        (Some(dir), None) if dir != "-f" => (dir, false),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    run(Path::new(&dir), force)
}

#[cfg(windows)]
fn run(dir: &Path, force: bool) -> ExitCode {
    let result = expand_home(dir).and_then(|dir| {
        prepare_dir(&dir, force)?;
        add_path::add_to_path(&dir)
    });
    match result {
        Ok(true) => println!("已加入用户 PATH；请从新终端使用"),
        Ok(false) => println!("PATH 已包含该目录"),
        Err(err) => {
            eprintln!("add-path: {err}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

#[cfg(windows)]
fn expand_home(dir: &Path) -> std::io::Result<std::path::PathBuf> {
    match dir.strip_prefix("~") {
        Ok(rest) => env::home_dir()
            .map(|home| home.join(rest))
            .ok_or_else(|| std::io::Error::other("找不到用户目录")),
        Err(_) => Ok(dir.to_path_buf()),
    }
}

#[cfg(windows)]
fn prepare_dir(dir: &Path, force: bool) -> std::io::Result<()> {
    if !force {
        return Ok(());
    }
    if !dir.is_absolute() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "需要绝对目录"));
    }
    std::fs::create_dir_all(dir)
}

#[cfg(not(windows))]
fn run(_: &Path, _: bool) -> ExitCode {
    eprintln!("add-path: 仅支持 Windows");
    ExitCode::FAILURE
}

#[cfg(all(windows, test))]
#[path = "../tests/cli.rs"]
mod tests;
