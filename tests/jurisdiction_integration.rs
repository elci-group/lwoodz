use std::process::Command;

#[test]
fn cli_binaries_agree_and_index_is_complete() {
    let dir = tempfile::tempdir().unwrap();
    let primary = Command::new(env!("CARGO_BIN_EXE_lwoodz"))
        .current_dir(dir.path())
        .args(["--json", "--jurisdictions"])
        .output()
        .unwrap();
    let companion = Command::new(env!("CARGO_BIN_EXE_lwoodz-cli"))
        .current_dir(dir.path())
        .args(["--json", "jurisdictions"])
        .output()
        .unwrap();
    assert!(
        primary.status.success(),
        "{}",
        String::from_utf8_lossy(&primary.stderr)
    );
    assert!(
        companion.status.success(),
        "{}",
        String::from_utf8_lossy(&companion.stderr)
    );
    let a: serde_json::Value = serde_json::from_slice(&primary.stdout).unwrap();
    let b: serde_json::Value = serde_json::from_slice(&companion.stdout).unwrap();
    assert_eq!(a, b);
    assert_eq!(a["schema_version"], lwoodz::jurisdiction::SCHEMA);
    let sections = a["jurisdictions"].as_array().unwrap();
    assert_eq!(sections.len(), lwoodz::jurisdiction::SUPPORTED.len());
    let eu = sections.iter().find(|j| j["jurisdiction"] == "EU").unwrap();
    assert_eq!(eu["consent_band"]["authorization_age"], 16);
    assert!(eu["certified_methods"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["id"] == "eu-scc-2021-914"));
    assert!(eu["certified_methods"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["id"] == "eu-us-data-privacy-framework"));
    let uk = sections.iter().find(|j| j["jurisdiction"] == "UK").unwrap();
    assert_eq!(uk["consent_band"]["coverage_age"], 18);
    assert!(uk["measures"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["id"] == "uk-childrens-code"));
    // The index dump must not touch the working directory.
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);

    let human = Command::new(env!("CARGO_BIN_EXE_lwoodz"))
        .current_dir(dir.path())
        .arg("--jurisdictions")
        .output()
        .unwrap();
    assert!(human.status.success());
    let text = String::from_utf8(human.stdout);
    let text = text.unwrap();
    for marker in [
        "Required measure",
        "Certified method",
        "Age band",
        "eu-transfer-safeguards",
        "us-coppa-safe-harbor",
    ] {
        assert!(text.contains(marker), "missing {marker}");
    }
}
