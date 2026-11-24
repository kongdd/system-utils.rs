use std::{fs, path::Path, process::Command};

fn lines(dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_lines"));
    cmd.current_dir(dir)
        .env("XDG_CONFIG_HOME", dir.join(".config"))
        .env("APPDATA", dir.join(".config"))
        .env_remove("CLICOLOR_FORCE");
    cmd
}

fn colored_output(dir: &Path) -> String {
    let output = lines(dir)
        .env_remove("NO_COLOR")
        .env("CLICOLOR_FORCE", "1")
        .args(["--exts", "rs"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn prints_matching_files_and_directory_totals() {
    let dir = tempfile::tempdir().unwrap();
    for (path, contents) in [
        ("a/nested/统计.R", &b"\r\n\xff"[..]),
        ("a/zero.rs", b""),
        ("b/skip.txt", b"ignored\n"),
        ("b/skip.r", b"ignored\n"),
        (".hidden/skip.rs", b"ignored\n"),
        ("main.rs", b"a\n\nb\n"),
    ] {
        let path = dir.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    let output = lines(dir.path()).args(["--exts", "rs,R"]).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        concat!(
            ".                   (5)\n",
            "├── a               (2)\n",
            "│   ├── nested      (2)\n",
            "│   │   └── 统计.R  (2) +\n",
            "│   └── zero.rs     (0)\n",
            "└── main.rs         (3) +\n",
        ),
    );
}

#[test]
fn scans_only_the_requested_directory() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("source code")).unwrap();
    fs::write(dir.path().join("source code/main.rs"), b"one\n").unwrap();
    fs::write(dir.path().join("outside.rs"), b"ignored\n").unwrap();

    let output = lines(dir.path())
        .env("NO_COLOR", "1")
        .env("CLICOLOR_FORCE", "1")
        .args(["-d", "source code", "--exts", "rs"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "source code  (1)\n└── main.rs  (1) +\n");
}

#[test]
fn marks_only_long_files_at_the_default_thresholds() {
    let dir = tempfile::tempdir().unwrap();
    for n in [299, 300, 600, 601] {
        fs::write(dir.path().join(format!("n{n}.rs")), b"\n".repeat(n)).unwrap();
    }

    let stdout = colored_output(dir.path());
    assert!(stdout.starts_with("\x1b[1;36m.\x1b[0m            \x1b[1m(1,800)\x1b[0m\n"));
    assert!(stdout.contains("\x1b[0mn299.rs\x1b[0m    (299) ++++++++++\x1b[0m"));
    assert!(stdout.contains("\x1b[1;33mn300.rs\x1b[0m    \x1b[1;33m(300) ++++++++++\x1b[0m"));
    assert!(
        stdout.contains("\x1b[1;33mn600.rs\x1b[0m    \x1b[1;33m(600) ++++++++++++++++++++\x1b[0m")
    );
    assert!(
        stdout.contains("\x1b[1;31mn601.rs\x1b[0m    \x1b[1;31m(601) ++++++++++++++++++++\x1b[0m")
    );
}

#[test]
fn lists_effective_thresholds() {
    let dir = tempfile::tempdir().unwrap();
    let list = || {
        let output = lines(dir.path()).args(["config", "list"]).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    };

    assert_eq!(list(), "n-warn = 300\nn-bad = 600\n");
    let status =
        lines(dir.path()).args(["config", "--n-warn", "100", "--n-bad", "200"]).status().unwrap();
    assert!(status.success());
    assert_eq!(list(), "n-warn = 100\nn-bad = 200\n");
}

#[test]
fn saves_custom_thresholds_and_rejects_invalid_updates() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("warn.rs"), b"\n\n").unwrap();
    fs::write(dir.path().join("bad.rs"), b"\n\n\n\n").unwrap();

    let status =
        lines(dir.path()).args(["config", "--n-warn", "2", "--n-bad", "3"]).status().unwrap();
    assert!(status.success());

    let output =
        lines(dir.path()).args(["config", "--n-warn", "4", "--n-bad", "3"]).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--n-warn must not exceed --n-bad"));

    let stdout = colored_output(dir.path());
    assert!(stdout.contains("\x1b[1;33mwarn.rs\x1b[0m"));
    assert!(stdout.contains("\x1b[1;31mbad.rs\x1b[0m"));
}

#[test]
fn inline_thresholds_default_to_warn_unless_a_filter_is_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let crates = dir.path().join("crates");
    fs::create_dir(&crates).unwrap();
    for (name, n) in [("short.rs", 1), ("warn.rs", 500), ("bad.rs", 1001)] {
        fs::write(crates.join(name), b"\n".repeat(n)).unwrap();
    }
    for (flags, expected) in [
        (&["--n-warn=100"][..], "warn.rs  (500) ++++++++++\n"),
        (&["--n-bad=500"], "warn.rs  (500) ++++++++++\n"),
        (&["--n-warn=100", "--bad"], "bad.rs  (1,001) ++++++++++++++++++++\n"),
    ] {
        let output =
            lines(dir.path()).args(flags).args(["-d", "crates", "--exts", "rs"]).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }
}

#[test]
fn filters_files_using_temporary_thresholds() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    for (path, n) in [("short.rs", 99), ("src/warn.rs", 100), ("notes.txt", 100), ("bad.rs", 1001)]
    {
        fs::write(dir.path().join(path), b"\n".repeat(n)).unwrap();
    }
    fs::write(dir.path().join("binary"), [b"\0".as_slice(), &b"\n".repeat(100)].concat()).unwrap();

    let run = |args: &[&str]| {
        let output = lines(dir.path()).args(args).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap().replace('\\', "/")
    };

    assert_eq!(run(&["--warn", "--n-warn", "100", "--exts", "rs"]), "src/warn.rs  (100) ++\n");
    assert_eq!(run(&["--bad"]), "bad.rs  (1,001) ++++++++++++++++++++\n");
    assert_eq!(run(&["--warn"]), "[ok] no warn file\n");
    assert_eq!(
        run(&["--warn", "--bad", "--n-warn", "100"]),
        "bad.rs     (1,001) ++++++++++++++++++++\nnotes.txt    (100) ++\nsrc/warn.rs  (100) ++\n"
    );
}

#[test]
fn highlights_empty_filter_results_in_bold_green() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("short.rs"), b"line\n").unwrap();

    for (flags, level) in
        [(&["--warn"][..], "warn"), (&["--bad"], "bad"), (&["--warn", "--bad"], "bad/warn")]
    {
        let output = lines(dir.path())
            .env_remove("NO_COLOR")
            .env("CLICOLOR_FORCE", "1")
            .args(["--exts", "rs"])
            .args(flags)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("\x1b[1m\x1b[32m[ok] no {level} file\x1b[0m\n")
        );
    }
}

#[test]
fn skips_test_directories_unless_requested() {
    let dir = tempfile::tempdir().unwrap();
    for path in ["src/main.rs", "src/test/unit.rs", "tests/integration.rs"] {
        let path = dir.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"line\n").unwrap();
    }

    for (flags, include_tests) in
        [(&["--exts", "rs"][..], false), (&["--exts", "rs", "--tests"], true)]
    {
        for mode in [&[][..], &["--n-warn=1"]] {
            let output = lines(dir.path()).args(flags).args(mode).output().unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            let stdout = String::from_utf8(output.stdout).unwrap();
            assert!(stdout.contains("main.rs"));
            assert_eq!(stdout.contains("unit.rs"), include_tests);
            assert_eq!(stdout.contains("integration.rs"), include_tests);
        }
    }
}

#[test]
fn honors_nested_gitignore_and_parent_rules() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src/cache")).unwrap();
    fs::create_dir(dir.path().join("src/.hidden")).unwrap();
    fs::write(dir.path().join(".gitignore"), "*.rs\ncache/\n").unwrap();
    fs::write(dir.path().join("src/.gitignore"), "!keep.rs\n!.hidden.rs\n!.hidden/\n").unwrap();
    fs::write(dir.path().join("src/cache/keep.rs"), b"ignored\n").unwrap();
    for name in ["keep.rs", "ignored.rs", ".hidden.rs", ".hidden/keep.rs"] {
        fs::write(dir.path().join("src").join(name), b"line\n").unwrap();
    }
    let output = lines(dir.path()).args(["-d", "src", "--exts", "rs"]).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout, "src          (1)\n└── keep.rs  (1) +\n");

    let output =
        lines(dir.path()).args(["--warn", "--n-warn", "1", "--exts", "rs"]).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().replace('\\', "/"),
        "src/keep.rs  (1) +\n"
    );
}

#[cfg(unix)]
#[test]
fn escapes_control_characters_before_aligning_counts() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a\nb.rs", "z.rs"] {
        fs::write(dir.path().join(name), b"line\n").unwrap();
    }
    for (flags, expected) in [
        (&[][..], ".            (2)\n├── a\\nb.rs  (1) +\n└── z.rs     (1) +\n"),
        (&["--n-warn=1"], "a\\nb.rs  (1) +\nz.rs     (1) +\n"),
    ] {
        let output = lines(dir.path()).args(["--exts", "rs"]).args(flags).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }
}

#[test]
fn rejects_missing_or_empty_extensions() {
    for args in [&[][..], &["--exts"], &["--exts", "rs,"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_lines")).args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("--exts"));
    }
}
