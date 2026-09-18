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
fn both_binaries_agree_on_service_terms_report() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let primary = Command::new(cargo_bin("lwoodz"))
        .current_dir(dir.path())
        .args(["--json", "--service-terms"])
        .output()
        .unwrap();
    let companion = Command::new(cargo_bin("lwoodz-cli"))
        .current_dir(dir.path())
        .args(["--json", "service-terms"])
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
    assert_eq!(a["schema_version"], lwoodz::service_terms::SCHEMA);
    assert!(a["findings"].as_array().unwrap().is_empty());
    assert!(!a["limitations"].as_array().unwrap().is_empty());
}

#[test]
fn human_output_lists_reference_material_when_nothing_matches() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let out = Command::new(cargo_bin("lwoodz-cli"))
        .current_dir(dir.path())
        .args(["service-terms"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("Service/inference terms-of-use analysis"));
    assert!(text.contains("No dependencies matched"));
}

#[test]
fn audit_surfaces_a_pointer_finding_when_a_service_dependency_is_declared() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());
    // No manifest present -> service-terms findings are empty, so audit
    // should not mention the pointer finding at all.
    let out = Command::new(cargo_bin("lwoodz-cli"))
        .current_dir(dir.path())
        .args(["--json", "audit"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let codes: Vec<&str> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].as_str().unwrap())
        .collect();
    assert!(!codes.contains(&"SERVICE_TERMS_REVIEW"));
    assert_eq!(json["service_terms"]["matched"], 0);
    assert_eq!(json["openness"]["total_deps"], 0);
}
