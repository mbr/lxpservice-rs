//! Verifies safety boundaries without authenticating against the service.

use std::process::Command;

/// Runs the CLI with synthetic credentials and isolated local storage.
fn cli() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lxp"));
    command
        .env("LXP_USERNAME", "dummy-user")
        .env("LXP_API_KEY", "must-not-appear-in-output")
        .env("LXP_MODE", "live")
        .env("RUST_LOG", "trace")
        .env_remove("LXP_STATE_DIR");
    command
}

/// Blocks environment-selected live uploads before reading files or creating receipts.
#[test]
fn live_mode_requires_separate_confirmation() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let result = cli()
        .args([
            "--state-dir",
            directory
                .path()
                .to_str()
                .expect("UTF-8 temporary directory"),
            "send",
            "nonexistent.pdf",
        ])
        .output()
        .expect("run CLI");
    assert!(!result.status.success());
    let diagnostics = String::from_utf8_lossy(&result.stderr);
    assert!(diagnostics.contains("requires --yes"));
    assert!(!diagnostics.contains("must-not-appear-in-output"));
    assert_eq!(
        std::fs::read_dir(directory.path())
            .expect("list receipts")
            .count(),
        0
    );
}

/// Keeps secrets out of help and rejects invalid PDFs before any upload.
#[test]
fn test_override_and_secret_safe_help() {
    let help = cli().arg("--help").output().expect("run help");
    assert!(help.status.success());
    assert!(!String::from_utf8_lossy(&help.stdout).contains("must-not-appear-in-output"));
    let directory = tempfile::tempdir().expect("temporary directory");
    let pdf = directory.path().join("invalid.pdf");
    std::fs::write(&pdf, b"not a PDF").expect("write invalid fixture");
    let result = cli()
        .args(["--mode", "test", "send"])
        .arg(pdf)
        .output()
        .expect("run validation");
    assert!(!result.status.success());
    let diagnostics = String::from_utf8_lossy(&result.stderr);
    assert!(diagnostics.contains("PDF header"));
    assert!(!diagnostics.contains("requires --yes"));
    assert!(!diagnostics.contains("must-not-appear-in-output"));
}
