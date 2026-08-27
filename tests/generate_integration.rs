// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

use std::io::Write;
use std::process::Command;

fn cargo_bin(name: &str) -> std::path::PathBuf {
    let exe = format!("{}{}", name, std::env::consts::EXE_SUFFIX);
    // Cargo sets CARGO_BIN_EXE_<name> for integration tests.
    if let Ok(p) = std::env::var(format!("CARGO_BIN_EXE_{}", name)) {
        return p.into();
    }
    // Fallback for running outside Cargo's test harness.
    std::path::PathBuf::from("target/debug").join(exe)
}

fn write_config(dir: &std::path::Path) {
    let cfg = r#"
[project]
license = "MIT"
copyright_holder = "Integration Test"
copyright_year = 2024
project_name = "test-project"
project_url = "https://example.org/test-project"

[operation]
mode = "observe"

[generate]
license_file = "LICENSE"
notice_file = "NOTICE"
copyright_file = "COPYRIGHT"
attribution_file = "THIRD_PARTY_NOTICES"
overwrite = true

[spdx]
produce_manifest = true
manifest_path = ".lwoodz/manifest.json"
"#;
    std::fs::write(dir.join("lwoodz.toml"), cfg).unwrap();
}

#[test]
fn lwoodz_cli_generate_dry_run_json() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let output = Command::new(cargo_bin("lwoodz-cli"))
        .args([
            "--config",
            dir.path().join("lwoodz.toml").to_str().unwrap(),
            "--json",
            "generate",
            "--dry-run",
        ])
        .output()
        .expect("lwoodz-cli should run");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["spdx"], "MIT");
    assert_eq!(json["holder"], "Integration Test");
    assert_eq!(json["year"], 2024);
    assert!(json["preview"].is_array());
}

#[test]
fn lwoodz_generate_dry_run_json() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let output = Command::new(cargo_bin("lwoodz"))
        .args([
            "--config",
            dir.path().join("lwoodz.toml").to_str().unwrap(),
            "--generate",
            "--dry-run",
            "--json",
        ])
        .output()
        .expect("lwoodz should run");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["spdx"], "MIT");
}

#[test]
fn both_binaries_generate_identical_license() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    // Run lwoodz --generate
    let out1 = Command::new(cargo_bin("lwoodz"))
        .args([
            "--config",
            dir.path().join("lwoodz.toml").to_str().unwrap(),
            "--generate",
            "--json",
        ])
        .output()
        .expect("lwoodz should run");
    assert!(
        out1.status.success(),
        "{}",
        String::from_utf8_lossy(&out1.stderr)
    );

    let body1 = std::fs::read_to_string(dir.path().join("LICENSE")).unwrap();

    // Clear the directory and re-run with lwoodz-cli generate
    std::fs::remove_file(dir.path().join("LICENSE")).unwrap();
    let out2 = Command::new(cargo_bin("lwoodz-cli"))
        .args([
            "--config",
            dir.path().join("lwoodz.toml").to_str().unwrap(),
            "generate",
        ])
        .output()
        .expect("lwoodz-cli should run");
    assert!(
        out2.status.success(),
        "{}",
        String::from_utf8_lossy(&out2.stderr)
    );

    let body2 = std::fs::read_to_string(dir.path().join("LICENSE")).unwrap();
    assert_eq!(body1, body2);
}

#[test]
fn generated_spdx_manifest_is_valid_spdx_23() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let output = Command::new(cargo_bin("lwoodz-cli"))
        .args([
            "--config",
            dir.path().join("lwoodz.toml").to_str().unwrap(),
            "--json",
            "generate",
        ])
        .output()
        .expect("lwoodz-cli should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let manifest_path = dir.path().join(".lwoodz/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();

    assert_eq!(manifest["spdxVersion"], "SPDX-2.3");
    assert_eq!(manifest["dataLicense"], "CC0-1.0");
    assert_eq!(manifest["SPDXID"], "SPDXRef-DOCUMENT");
    assert!(manifest["name"].is_string());
    assert!(manifest["documentNamespace"]
        .as_str()
        .unwrap()
        .starts_with("https://"));

    let packages = manifest["packages"].as_array().unwrap();
    assert!(!packages.is_empty());
    let project = packages
        .iter()
        .find(|p| p["SPDXID"] == "SPDXRef-Package-Project")
        .expect("project package missing");
    assert_eq!(project["name"], "test-project");
    assert_eq!(project["licenseConcluded"], "MIT");
    assert_eq!(
        project["downloadLocation"],
        "https://example.org/test-project"
    );

    let relationships = manifest["relationships"].as_array().unwrap();
    assert!(relationships.iter().any(|r| {
        r["spdxElementId"] == "SPDXRef-DOCUMENT"
            && r["relationshipType"] == "DESCRIBES"
            && r["relatedSpdxElement"] == "SPDXRef-Package-Project"
    }));
}

#[test]
fn overwrite_false_keeps_existing_files() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let output = Command::new(cargo_bin("lwoodz-cli"))
        .args([
            "--config",
            dir.path().join("lwoodz.toml").to_str().unwrap(),
            "generate",
        ])
        .output()
        .expect("lwoodz-cli should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Mutate the generated LICENSE.
    std::fs::write(dir.path().join("LICENSE"), "STALE").unwrap();

    // Toggle overwrite off by rewriting config.
    let mut cfg_file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(dir.path().join("lwoodz.toml"))
        .unwrap();
    writeln!(
        cfg_file,
        r#"
[project]
license = "MIT"
copyright_holder = "Integration Test"
copyright_year = 2024
project_name = "test-project"

[operation]
mode = "observe"

[generate]
license_file = "LICENSE"
notice_file = "NOTICE"
copyright_file = "COPYRIGHT"
attribution_file = "THIRD_PARTY_NOTICES"
overwrite = false

[spdx]
produce_manifest = true
manifest_path = ".lwoodz/manifest.json"
"#
    )
    .unwrap();

    let output = Command::new(cargo_bin("lwoodz-cli"))
        .args([
            "--config",
            dir.path().join("lwoodz.toml").to_str().unwrap(),
            "--json",
            "generate",
        ])
        .output()
        .expect("lwoodz-cli should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["skipped"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p.as_str().unwrap().ends_with("LICENSE")));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("LICENSE")).unwrap(),
        "STALE"
    );
}
