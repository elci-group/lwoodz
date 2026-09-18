// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
use std::process::Command;

fn cargo_bin(name: &str) -> std::path::PathBuf {
    match name {
        "lwoodz" => env!("CARGO_BIN_EXE_lwoodz").into(),
        "lwoodz-cli" => env!("CARGO_BIN_EXE_lwoodz-cli").into(),
        _ => panic!("unknown test binary"),
    }
}

fn write_config(dir: &std::path::Path) {
    let cfg = r#"
[project]
license = "MIT"
copyright_holder = "Integration Test"
copyright_year = 2024
project_name = "test-project"
"#;
    std::fs::write(dir.join("lwoodz.toml"), cfg).unwrap();
}

#[test]
fn both_binaries_agree_on_openness_report() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let primary = Command::new(cargo_bin("lwoodz"))
        .current_dir(dir.path())
        .args(["--json", "--openness"])
        .output()
        .unwrap();
    let companion = Command::new(cargo_bin("lwoodz-cli"))
        .current_dir(dir.path())
        .args(["--json", "openness"])
        .output()
        .unwrap();

    assert!(
        primary.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&primary.stderr)
    );
    assert!(
        companion.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&companion.stderr)
    );

    let a: serde_json::Value = serde_json::from_slice(&primary.stdout).unwrap();
    let b: serde_json::Value = serde_json::from_slice(&companion.stdout).unwrap();
    assert_eq!(a, b);
    assert_eq!(a["schema_version"], lwoodz::openness::SCHEMA);
    assert!(a["dependencies"].as_array().unwrap().is_empty());
}

#[test]
fn human_output_reports_differentiation_summary() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let out = Command::new(cargo_bin("lwoodz-cli"))
        .current_dir(dir.path())
        .args(["openness"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("Open-source / open-standard differentiation"));
    assert!(text.contains("0 dependencies"));
}
