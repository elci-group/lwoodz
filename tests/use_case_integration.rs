use lwoodz::use_case::{self, Activity, Profiles};
use std::process::Command;

fn sample() -> Profiles {
    toml::from_str(include_str!("../examples/use-case.toml")).unwrap()
}
fn ids(t: &use_case::TargetAnalysis) -> Vec<&str> {
    t.recommendations.iter().map(|r| r.id.as_str()).collect()
}
#[test]
fn market_and_activity_rules_are_isolated_per_target() {
    let report = use_case::analyze(&sample()).unwrap();
    assert!(ids(&report.targets[0]).contains(&"CHILDREN"));
    assert!(ids(&report.targets[0]).contains(&"NETWORK_COPYLEFT"));
    assert!(!ids(&report.targets[0]).contains(&"DISTRIBUTION"));
    assert!(ids(&report.targets[1]).contains(&"DISTRIBUTION"));
    assert!(!ids(&report.targets[1]).contains(&"HOSTING"));
    assert!(!ids(&report.targets[1]).contains(&"CHILDREN"));
    assert!(report.targets[1].unknowns.iter().any(|s| s.contains("JP")));
    for t in &report.targets {
        for r in &t.recommendations {
            assert_eq!(r.directive.target, t.profile.target);
            assert!(!r.directive.acceptance_tests.is_empty());
            for id in &r.source_ids {
                assert!(report.sources.iter().any(|s| &s.id == id));
            }
        }
    }
}
#[test]
fn missing_facts_are_not_treated_as_false() {
    let mut p = sample();
    p.targets.truncate(1);
    p.targets[0].children = None;
    let r = use_case::analyze(&p).unwrap();
    assert!(r.targets[0].unknowns.iter().any(|s| s.contains("children")));
    assert!(ids(&r.targets[0]).contains(&"SCOPE_REVIEW"));
    assert!(!ids(&r.targets[0]).contains(&"CHILDREN"));
}
#[test]
fn invalid_and_misspelled_profiles_fail() {
    let mut p = sample();
    p.targets[0].sensitive_data = Some(true);
    p.targets[0].personal_data = Some(false);
    assert!(use_case::analyze(&p).is_err());
    let mut p = sample();
    p.targets.push(p.targets[0].clone());
    assert!(p.validate().is_err());
    assert!(toml::from_str::<Profiles>(
        &include_str!("../examples/use-case.toml").replace("children =", "chidlren =")
    )
    .is_err());
    assert!(use_case::analyze(&Profiles { targets: vec![] }).is_err());
}
#[test]
fn hosting_is_required_for_network_copyleft_review() {
    let mut p = sample();
    p.targets[0].activities = vec![Activity::ShareSource];
    assert!(!ids(&use_case::analyze(&p).unwrap().targets[0]).contains(&"NETWORK_COPYLEFT"));
    p.targets[0].activities = vec![Activity::Host];
    p.targets[0].dependency_licenses = vec!["MIT OR AGPL-3.0-only".into()];
    let r = use_case::analyze(&p).unwrap();
    let agpl = r.targets[0]
        .recommendations
        .iter()
        .find(|r| r.id == "NETWORK_COPYLEFT")
        .unwrap();
    assert!(agpl.applicability.contains("selected expression branch"));
}
#[test]
fn jurisdiction_roles_and_aliases_survive_analysis() {
    let mut p = sample();
    p.targets[0].user_jurisdictions = vec![" uk ".into(), "GB".into()];
    p.targets[0].developer_jurisdictions = vec!["UK".into()];
    p.targets[0].hosting_jurisdictions = vec!["UK".into()];
    p.targets[0].cross_border_transfers = Some(false);
    let r = use_case::analyze(&p).unwrap();
    assert_eq!(r.targets[0].jurisdiction_reviews.len(), 1);
    assert_eq!(r.targets[0].jurisdiction_reviews[0].connections.len(), 3);
    assert!(!ids(&r.targets[0]).contains(&"TRANSFERS"));
}
#[test]
fn age_gate_detection_and_required_measures_follow_declarations() {
    let report = use_case::analyze(&sample()).unwrap();
    let school = &report.targets[0];
    assert_eq!(school.age_gate.minimum_age, Some(13));
    assert!(
        school.age_gate.blocked,
        "children=true makes band breaches P0"
    );
    let codes: Vec<&str> = school
        .age_gate
        .findings
        .iter()
        .map(|f| f.code.as_str())
        .collect();
    assert!(codes.contains(&"BELOW_AUTHORIZATION_BAND"));
    assert!(codes.contains(&"SELF_DECLARED_GATE_INSUFFICIENT"));
    assert!(codes.contains(&"CHILD_PROTECTION_CODE_SCOPE"));
    assert!(!codes.contains(&"AGE_GATE_MISSING"));
    assert!(!codes.contains(&"AGE_SCOPE_UNKNOWN"));
    assert_eq!(school.age_gate.bands.len(), 3);

    let measure_ids: Vec<&str> = school
        .required_measures
        .iter()
        .map(|m| m.id.as_str())
        .collect();
    for expected in [
        "eu-transfer-safeguards",
        "eu-child-authorization",
        "uk-restricted-transfer-route",
        "uk-child-authorization",
        "uk-childrens-code",
        "us-ca-minor-opt-in",
        "us-ca-consumer-rights",
    ] {
        assert!(measure_ids.contains(&expected), "missing {expected}");
    }
    // COPPA is not triggered: the US is not among the declared jurisdictions.
    assert!(!measure_ids.contains(&"us-coppa-parental-consent"));
    // Declaring "US" surfaces COPPA with its certified safe harbor method.
    let mut p = sample();
    p.targets[0].user_jurisdictions = vec!["US".into()];
    let us_report = use_case::analyze(&p).unwrap();
    let coppa = us_report.targets[0]
        .required_measures
        .iter()
        .find(|m| m.id == "us-coppa-parental-consent")
        .unwrap();
    assert!(coppa
        .certified_methods
        .iter()
        .any(|m| m.id == "us-coppa-safe-harbor"));
    // Transfer measures carry their certified instruments.
    let transfer = school
        .required_measures
        .iter()
        .find(|m| m.id == "eu-transfer-safeguards")
        .unwrap();
    assert!(transfer
        .certified_methods
        .iter()
        .any(|m| m.id == "eu-scc-2021-914"));
    // Child consent has no certified method on record; empty means "none
    // maintained", not "optional".
    let uk_child = school
        .required_measures
        .iter()
        .find(|m| m.id == "uk-child-authorization")
        .unwrap();
    assert!(uk_child.certified_methods.is_empty());

    // The offline SDK declares no personal data, no children, no transfers:
    // no measures trigger and the age gate is clean.
    let sdk = &report.targets[1];
    assert!(sdk.age_gate.findings.is_empty());
    assert!(!sdk.age_gate.blocked);
    assert!(sdk.required_measures.is_empty());
}
#[test]
fn cli_binaries_agree_and_do_not_write_to_target() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.toml");
    std::fs::write(&path, include_str!("../examples/use-case.toml")).unwrap();
    let primary = Command::new(env!("CARGO_BIN_EXE_lwoodz"))
        .current_dir(dir.path())
        .args(["--json", "--analyze"])
        .arg(&path)
        .output()
        .unwrap();
    let companion = Command::new(env!("CARGO_BIN_EXE_lwoodz-cli"))
        .current_dir(dir.path())
        .args(["--json", "analyze", "--profile"])
        .arg(&path)
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
    assert_eq!(a["schema_version"], use_case::SCHEMA);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    let path_json = dir.path().join("profile.json");
    std::fs::write(&path_json, serde_json::to_vec(&sample()).unwrap()).unwrap();
    assert_eq!(use_case::load(&path_json).unwrap().targets.len(), 2);
    let human = Command::new(env!("CARGO_BIN_EXE_lwoodz-cli"))
        .current_dir(dir.path())
        .args(["analyze", "--profile"])
        .arg(&path_json)
        .output()
        .unwrap();
    assert!(human.status.success());
    let text = String::from_utf8(human.stdout).unwrap();
    for marker in [
        "User risk:",
        "Developer exposure:",
        "Residual risk:",
        "Acceptance:",
        "Jurisdiction",
        "Age gate:",
        "Age band",
        "Age finding",
        "Required measures:",
        "Certified method",
        "References:",
    ] {
        assert!(text.contains(marker));
    }
}
#[test]
fn cli_rejects_invalid_profile_without_partial_report() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.toml");
    std::fs::write(&path, "targets = []").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_lwoodz-cli"))
        .current_dir(dir.path())
        .args(["--json", "analyze", "--profile"])
        .arg(path)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
}
