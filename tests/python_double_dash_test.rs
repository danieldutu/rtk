//! Exercise clap, dispatch and the child argv together: a leading separator
//! must reach the tool, after pytest's injected options, without routing a
//! protected word such as `list`, `run` or `lint` as a subcommand.
#![cfg(unix)]

mod common;

use std::os::unix::fs::PermissionsExt;

fn check_tool(tool: &str, injected: &[&str]) {
    let dir = common::tempdir();
    let capture = dir.path().join("argv");
    let stub = dir.path().join(tool);
    std::fs::write(
        &stub,
        "#!/bin/sh\nprintf '%s\\0' \"$@\" > \"$ARGV_CAPTURE\"\nexit \"$STUB_EXIT\"\n",
    )
    .expect("write stub");
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).expect("chmod stub");
    let path = std::env::join_paths(std::iter::once(dir.path().to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .expect("PATH");

    let cases: &[&[&str]] = &[
        &["--", "-k.py"],
        &["--"],
        &["--", "--"],
        &["--", "--", "-k.py"],
        &["--", "list"],
        &["--", "run"],
        &["--", "lint"],
        &["zzz", "--", "-k.py"],
        &["zzz"],
    ];
    for args in cases {
        for code in [0, 7] {
            // Each process has fresh argv evidence, even if dispatch fails.
            let _ = std::fs::remove_file(&capture);
            let output = common::rtk_command()
                .current_dir(dir.path())
                .env("PATH", &path)
                .env("ARGV_CAPTURE", &capture)
                .env("STUB_EXIT", code.to_string())
                .args(["-v", tool])
                .args(*args)
                .output()
                .expect("run rtk");
            assert_eq!(
                output.status.code(),
                Some(code),
                "{tool} {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let bytes = std::fs::read(&capture).expect("tool must execute");
            let actual: Vec<&[u8]> = bytes
                .strip_suffix(&[0])
                .expect("NUL-terminated argv")
                .split(|b| *b == 0)
                .collect();
            let expected: Vec<&[u8]> = injected
                .iter()
                .chain(args.iter())
                .map(|s| s.as_bytes())
                .collect();
            assert_eq!(actual, expected, "{tool} {args:?}");
        }
    }
}

#[test]
fn pytest_keeps_separator_after_injected_options() {
    check_tool("pytest", &["--tb=short", "-q", "-rxX"]);
}

#[test]
fn mypy_keeps_separator() {
    check_tool("mypy", &[]);
}

#[test]
fn pip_keeps_separator_before_subcommand_like_paths() {
    check_tool("pip", &[]);
}

#[test]
fn uv_keeps_separator_before_subcommand_like_paths() {
    check_tool("uv", &[]);
}

#[test]
fn sqlfluff_keeps_separator_before_subcommand_like_paths() {
    check_tool("sqlfluff", &[]);
}
