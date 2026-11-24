use std::{env, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    match (args.next(), args.next()) {
        (Some(arg), None) if arg == "-h" || arg == "--help" => {
            println!("用法：add-path <绝对目录>");
            ExitCode::SUCCESS
        }
        #[cfg(windows)]
        (Some(dir), None) => match add_path::add_to_path(std::path::Path::new(&dir)) {
            Ok(true) => {
                println!("已加入用户 PATH；请从新终端使用");
                ExitCode::SUCCESS
            }
            Ok(false) => {
                println!("PATH 已包含该目录");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("add-path: {err}");
                ExitCode::FAILURE
            }
        },
        #[cfg(not(windows))]
        (Some(_), None) => {
            eprintln!("add-path: 仅支持 Windows");
            ExitCode::FAILURE
        }
        _ => {
            eprintln!("用法：add-path <绝对目录>");
            ExitCode::from(2)
        }
    }
}
