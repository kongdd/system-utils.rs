use std::process::Command;

#[test]
fn rejects_invalid_arguments() {
    let cases: &[&[&str]] = &[
        &[],
        &["--unknown"],
        &["--stdout", "extra"],
        &["--output"],
        &["--output", "shot.png", "extra"],
    ];
    let expected =
        b"clipboard-image: usage: clipboard-image --stdout | clipboard-image --output PATH\n";

    for args in cases {
        let output = Command::new(env!("CARGO_BIN_EXE_clipboard-image"))
            .args(*args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, expected, "{args:?}");
    }
}
