use std::path::Path;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_audio-player");
const USAGE: &str = "Usage: audio-player <file.mp3>\n";

fn run(args: &[&str]) -> Output {
    Command::new(BIN).args(args).output().unwrap()
}

#[test]
fn prints_help() {
    for flag in ["-h", "--help"] {
        let output = run(&[flag]);
        assert!(output.status.success());
        assert_eq!(output.stdout, USAGE.as_bytes());
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn prints_version() {
    let version = concat!("audio-player ", env!("CARGO_PKG_VERSION"), "\n");
    for flag in ["-V", "--version"] {
        let output = run(&[flag]);
        assert!(output.status.success());
        assert_eq!(output.stdout, version.as_bytes());
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn rejects_wrong_argument_count() {
    for args in [&[][..], &["--help", "extra"]] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, USAGE.as_bytes());
    }
}

#[test]
fn reports_unreadable_input() {
    // A child of the executable file cannot exist.
    let path = Path::new(BIN).join("missing audio.mp3");
    let output = Command::new(BIN).arg(&path).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(
        error.starts_with(&format!("audio-player: {}: ", path.display())),
        "{error}"
    );
}
