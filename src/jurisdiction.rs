// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
//! Global per-jurisdiction indices of required measures and certified
//! methods of implementation, with the consent bands that drive age gate
//! detection.
//!
//! A *required measure* is an obligation a jurisdiction attaches to a
//! declared activity (e.g. putting a transfer safeguard in place before
//! personal data leaves the EU/UK, or obtaining verifiable parental consent
//! before collecting data from under-13s in the US). A *certified method* is
//! a mechanism an authority has approved, recognized or codified for
//! implementing one of those measures (EU Standard Contractual Clauses, the
//! UK IDTA, the EU-US Data Privacy Framework, FTC-approved COPPA safe harbor
//! programs, the recognized Global Privacy Control signal).
//!
//! The tables here are a dated, offline reference set — the same epistemic
//! contract as [`crate::service_terms`]'s catalog: they record what the
//! reviewed sources say, they never measure actual behavior, and they are
//! not legal advice. Requirement *detection* (matching these indices against
//! a declared profile) lives in [`crate::use_case`], as does age gate
//! detection; this module owns the indices themselves so both sides read
//! from one table.
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const SCHEMA: &str = "lwoodz.jurisdiction-index/v1";
pub const REVIEWED: &str = "2026-10-03";

/// Jurisdiction codes with maintained index entries. Anything else is an
/// explicit coverage gap (see the unsupported-jurisdiction review in
/// [`crate::use_case`]).
pub const SUPPORTED: &[&str] = &["EU", "UK", "US", "US-CA"];

/// Normalize a declared jurisdiction code to its canonical index code.
/// Aliases fold onto maintained entries; unknown codes are preserved
/// uppercase so coverage gaps stay visible.
pub fn canonical(value: &str) -> String {
    match value.trim().to_uppercase().as_str() {
        "EEA" | "EU" => "EU".into(),
        "GB" | "UK" => "UK".into(),
        v => v.into(),
    }
}

/// Declared profile facts a measure trigger is evaluated against. Derived
/// from a use-case profile once per target; omitted (`None`) facts never
/// satisfy a trigger — unknown is not true.
#[derive(Debug, Clone, Copy, Default)]
pub struct MeasureFacts {
    pub children: Option<bool>,
    pub minimum_age: Option<u8>,
    pub personal_data: Option<bool>,
    pub sensitive_data: Option<bool>,
    pub automated_decisions: Option<bool>,
    pub user_content: Option<bool>,
    /// Cross-border transfer declared, or user/hosting territories differ.
    pub transfers: bool,
    /// Operate or Host activity: the target runs a service users interact
    /// with (as opposed to only shipping artifacts).
    pub operates_service: bool,
}

/// The profile condition that makes a measure applicable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasureTrigger {
    /// Personal data crosses a border (declared, or user vs hosting
    /// territories differ) while processing is not excluded.
    CrossBorderTransfer,
    /// The audience includes children, or the declared minimum age sits
    /// below the jurisdiction's parental-authorization age.
    ChildAuthorization { below: u8 },
    /// A service is likely to be accessed by minors below the given age
    /// (child-protection design codes), whether or not they are the
    /// intended audience.
    ServiceAccessedByChildren { below: u8 },
    /// Sensitive data is declared.
    SensitiveData,
    /// Automated decisions are declared.
    AutomatedDecisions,
    /// Personal data of consumers is declared.
    ConsumerPrivacyRights,
    /// User content is declared on a service the operator runs.
    UserContentService,
}

/// Who stands behind a certified method: the approving or recognizing
/// authority named by the reviewed instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MethodStatus {
    /// Formally approved by the named authority (adequacy decision,
    /// approved contractual clauses, approved safe harbor program).
    Approved,
    /// Recognized as a valid mechanism by regulation or regulator guidance.
    Recognized,
    /// A statutory code of practice the named authority enforces.
    StatutoryCode,
}

impl MethodStatus {
    /// Snake-case label used in reports.
    pub fn label(&self) -> &'static str {
        match self {
            MethodStatus::Approved => "approved",
            MethodStatus::Recognized => "recognized",
            MethodStatus::StatutoryCode => "statutory_code",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredMeasure {
    pub id: String,
    pub jurisdiction: String,
    pub name: String,
    pub trigger: MeasureTrigger,
    /// What the obligated party must do, in obligation terms.
    pub obligation: String,
    pub legal_basis: String,
    /// Source id in the use-case analysis `sources` table.
    pub source_id: String,
    pub reviewed_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertifiedMethod {
    pub id: String,
    pub jurisdiction: String,
    pub name: String,
    /// Id of the [`RequiredMeasure`] this method implements.
    pub implements: String,
    pub certifier: String,
    /// The instrument that establishes the method (decision number,
    /// regulation citation, code title).
    pub instrument: String,
    pub status: MethodStatus,
    pub reference_url: String,
    pub reviewed_on: String,
}

/// Age boundary a jurisdiction attaches to child-protection rules:
/// below `authorization_age` a parent/guardian must authorize collection;
/// below `coverage_age` the wider child-protection design rules scope in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentBand {
    pub jurisdiction: String,
    pub authorization_age: u8,
    pub coverage_age: u8,
    pub rule: String,
    pub source_id: String,
    pub reviewed_on: String,
}

impl RequiredMeasure {
    /// Evaluate this measure against declared facts. Returns the human
    /// readable trigger reasons when the measure applies, `None` otherwise.
    pub fn applies(&self, facts: &MeasureFacts) -> Option<Vec<String>> {
        let child_scope = |below: u8| -> Option<Vec<String>> {
            if facts.children == Some(true) {
                return Some(vec![format!(
                    "children declared in the audience (under-{below} rules in scope)"
                )]);
            }
            if let Some(min) = facts.minimum_age {
                if min < below {
                    return Some(vec![format!(
                        "minimum_age {min} is below the {below}+ authorization boundary"
                    )]);
                }
            }
            None
        };
        let reasons = match self.trigger {
            MeasureTrigger::CrossBorderTransfer => {
                if facts.transfers {
                    vec!["cross-border transfer declared or user/hosting territories differ".into()]
                } else {
                    return None;
                }
            }
            MeasureTrigger::ChildAuthorization { below } => child_scope(below)?,
            MeasureTrigger::ServiceAccessedByChildren { below } => child_scope(below)?,
            MeasureTrigger::SensitiveData => {
                if facts.sensitive_data == Some(true) {
                    vec!["sensitive_data declared".into()]
                } else {
                    return None;
                }
            }
            MeasureTrigger::AutomatedDecisions => {
                if facts.automated_decisions == Some(true) {
                    vec!["automated_decisions declared".into()]
                } else {
                    return None;
                }
            }
            MeasureTrigger::ConsumerPrivacyRights => {
                if facts.personal_data == Some(true) {
                    vec!["personal_data declared".into()]
                } else {
                    return None;
                }
            }
            MeasureTrigger::UserContentService => {
                if facts.user_content == Some(true) && facts.operates_service {
                    vec!["user_content declared on an operated or hosted service".into()]
                } else {
                    return None;
                }
            }
        };
        Some(reasons)
    }
}

fn measure_entry(
    id: &str,
    jurisdiction: &str,
    name: &str,
    trigger: MeasureTrigger,
    obligation: &str,
    legal_basis: &str,
    source_id: &str,
) -> RequiredMeasure {
    RequiredMeasure {
        id: id.into(),
        jurisdiction: jurisdiction.into(),
        name: name.into(),
        trigger,
        obligation: obligation.into(),
        legal_basis: legal_basis.into(),
        source_id: source_id.into(),
        reviewed_on: REVIEWED.into(),
    }
}

fn method(
    id: &str,
    jurisdiction: &str,
    name: &str,
    implements: &str,
    certifier: &str,
    instrument: &str,
    status: MethodStatus,
    reference_url: &str,
) -> CertifiedMethod {
    CertifiedMethod {
        id: id.into(),
        jurisdiction: jurisdiction.into(),
        name: name.into(),
        implements: implements.into(),
        certifier: certifier.into(),
        instrument: instrument.into(),
        status,
        reference_url: reference_url.into(),
        reviewed_on: REVIEWED.into(),
    }
}

fn band_entry(
    jurisdiction: &str,
    authorization_age: u8,
    coverage_age: u8,
    rule: &str,
    source_id: &str,
) -> ConsentBand {
    ConsentBand {
        jurisdiction: jurisdiction.into(),
        authorization_age,
        coverage_age,
        rule: rule.into(),
        source_id: source_id.into(),
        reviewed_on: REVIEWED.into(),
    }
}

/// The maintained index of required measures, one entry per
/// jurisdiction/obligation pair. Extend this table (and bump [`REVIEWED`])
/// when a maintained source changes; do not widen triggers inline in
/// analysis code.
fn tables() -> (
    &'static [RequiredMeasure],
    &'static [CertifiedMethod],
    &'static [ConsentBand],
) {
    static TABLES: OnceLock<(Vec<RequiredMeasure>, Vec<CertifiedMethod>, Vec<ConsentBand>)> =
        OnceLock::new();
    let (measures, methods, bands) = TABLES.get_or_init(|| {
        (
            vec![
                // EU -----------------------------------------------------------------
                measure_entry(
                    "eu-transfer-safeguards",
                    "EU",
                    "International transfer safeguards",
                    MeasureTrigger::CrossBorderTransfer,
                    "Put an approved safeguard in place before personal data moves to a third country or international organization, including vendor processing and remote support access; document the transfer assessment.",
                    "GDPR Arts. 44–49",
                    "gdpr",
                ),
                measure_entry(
                    "eu-child-authorization",
                    "EU",
                    "Child consent with parental authorization",
                    MeasureTrigger::ChildAuthorization { below: 16 },
                    "Where an information-society service is offered directly to a child, obtain the holder of parental responsibility's consent below the applicable member-state age (13–16) and make reasonable efforts to verify it.",
                    "GDPR Art. 8",
                    "gdpr",
                ),
                measure_entry(
                    "eu-sensitive-data-conditions",
                    "EU",
                    "Sensitive data processing conditions and DPIA",
                    MeasureTrigger::SensitiveData,
                    "Process special categories only under an Article 9 condition, and assess whether a DPIA is required before high-risk processing starts.",
                    "GDPR Arts. 9, 35",
                    "gdpr",
                ),
                measure_entry(
                    "eu-automated-decision-conditions",
                    "EU",
                    "Automated decision safeguards",
                    MeasureTrigger::AutomatedDecisions,
                    "Give data subjects the right to human intervention, to express their point of view and to contest decisions with legal or similarly significant effect, except where a narrow exception applies.",
                    "GDPR Art. 22",
                    "gdpr",
                ),
                measure_entry(
                    "eu-notice-and-action",
                    "EU",
                    "Illegal content notice-and-action",
                    MeasureTrigger::UserContentService,
                    "Provide a notice mechanism for illegal content, act on notices with stated timing and reasons, and apply the transparency and statement-of-reasons duties that apply to the service's role and size.",
                    "Regulation (EU) 2022/2065 (DSA) Arts. 14–17, 20",
                    "dsa",
                ),
                // UK -----------------------------------------------------------------
                measure_entry(
                    "uk-restricted-transfer-route",
                    "UK",
                    "Restricted transfer route",
                    MeasureTrigger::CrossBorderTransfer,
                    "Ensure a restricted transfer from the UK uses the UK IDTA, the UK Addendum to EU SCCs, UK binding corporate rules or a UK adequacy regulation, supported by a transfer risk assessment.",
                    "UK GDPR/DP Act 2018 transfer provisions",
                    "uk-transfer",
                ),
                measure_entry(
                    "uk-child-authorization",
                    "UK",
                    "Child consent (fixed age 13)",
                    MeasureTrigger::ChildAuthorization { below: 13 },
                    "Obtain consent from the holder of parental responsibility for a child below 13 where the UK GDPR child-consent rule applies to the online service.",
                    "UK GDPR Art. 8; DP Act 2018 s.125",
                    "uk-gdpr",
                ),
                measure_entry(
                    "uk-childrens-code",
                    "UK",
                    "Age-appropriate design (Children's Code)",
                    MeasureTrigger::ServiceAccessedByChildren { below: 18 },
                    "Apply the Children's Code standards to online services likely to be accessed by under-18s: age-appropriate defaults, data minimization, profiling off by default, parental controls and transparency.",
                    "DP Act 2018 s.123 ICO statutory code",
                    "uk-childrens-code",
                ),
                // US -----------------------------------------------------------------
                measure_entry(
                    "us-coppa-parental-consent",
                    "US",
                    "COPPA verifiable parental consent",
                    MeasureTrigger::ChildAuthorization { below: 13 },
                    "For services directed to children under 13, or with actual knowledge of collecting their personal information, obtain verifiable parental consent before collection and honor ongoing parental rights over that data.",
                    "COPPA, 15 U.S.C. §§ 6501–6506; 16 CFR Part 312",
                    "coppa",
                ),
                // US-CA --------------------------------------------------------------
                measure_entry(
                    "us-ca-consumer-rights",
                    "US-CA",
                    "California consumer rights",
                    MeasureTrigger::ConsumerPrivacyRights,
                    "Honor CCPA/CPRA rights for covered activity: access, deletion, correction, portability and opt-out of sale/sharing, with the required notices and request pipeline.",
                    "Cal. Civ. Code § 1798.100 et seq. (CCPA/CPRA)",
                    "ccpa",
                ),
                measure_entry(
                    "us-ca-minor-opt-in",
                    "US-CA",
                    "Minor opt-in for sale/sharing",
                    MeasureTrigger::ChildAuthorization { below: 16 },
                    "Do not sell or share personal information of consumers under 16 without opt-in: affirmative authorization of the parent or guardian for a consumer under 13, and of the consumer aged 13–15.",
                    "Cal. Civ. Code § 1798.120(c)",
                    "ccpa",
                ),
            ],
            vec![
                method(
                    "eu-scc-2021-914",
                    "EU",
                    "EU Standard Contractual Clauses (module-based SCCs)",
                    "eu-transfer-safeguards",
                    "European Commission",
                    "Commission Implementing Decision (EU) 2021/914",
                    MethodStatus::Approved,
                    "https://eur-lex.europa.eu/eli/dec_impl/2021/914/oj",
                ),
                method(
                    "eu-binding-corporate-rules",
                    "EU",
                    "Binding Corporate Rules (intragroup)",
                    "eu-transfer-safeguards",
                    "Competent EU supervisory authority",
                    "GDPR Art. 47 authorization",
                    MethodStatus::Approved,
                    "https://eur-lex.europa.eu/eli/reg/2016/679/oj/eng",
                ),
                method(
                    "eu-us-data-privacy-framework",
                    "EU",
                    "EU-US Data Privacy Framework (adequacy)",
                    "eu-transfer-safeguards",
                    "European Commission",
                    "Implementing Decision (EU) 2023/1795",
                    MethodStatus::Approved,
                    "https://commission.europa.eu/law/law-topic/data-protection/international-dimension-data-protection/eu-us-data-privacy-framework_en",
                ),
                method(
                    "uk-idta-addendum",
                    "UK",
                    "UK International Data Transfer Agreement / UK Addendum to EU SCCs",
                    "uk-restricted-transfer-route",
                    "Information Commissioner's Office",
                    "ICO IDTA and UK Addendum (2022)",
                    MethodStatus::Approved,
                    "https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/international-transfers/international-data-transfer-agreement-and-guidance/",
                ),
                method(
                    "uk-adequacy-regulations",
                    "UK",
                    "UK adequacy regulations",
                    "uk-restricted-transfer-route",
                    "UK Secretary of State",
                    "Adequacy regulations made under the DP Act 2018",
                    MethodStatus::Approved,
                    "https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/international-transfers/a-guide-to-international-transfers/",
                ),
                method(
                    "uk-childrens-code-standards",
                    "UK",
                    "Children's Code standards as the design baseline",
                    "uk-childrens-code",
                    "Information Commissioner's Office",
                    "Age-appropriate design code of practice (2020, as revised)",
                    MethodStatus::StatutoryCode,
                    "https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/childrens-code-guidance-and-resources/",
                ),
                method(
                    "us-coppa-safe-harbor",
                    "US",
                    "FTC-approved COPPA safe harbor program",
                    "us-coppa-parental-consent",
                    "US Federal Trade Commission",
                    "16 CFR § 312.11(c) self-regulatory safe harbor guidelines",
                    MethodStatus::Approved,
                    "https://www.ftc.gov/legal-library/browse/rules/childrens-online-privacy-protection-rule-coppa",
                ),
                method(
                    "us-ca-global-privacy-control",
                    "US-CA",
                    "Global Privacy Control opt-out preference signal",
                    "us-ca-consumer-rights",
                    "California Privacy Protection Agency",
                    "CCPA regulations § 7025 (opt-out preference signals)",
                    MethodStatus::Recognized,
                    "https://oag.ca.gov/privacy/ccpa",
                ),
            ],
            vec![
                band_entry(
                    "EU",
                    16,
                    16,
                    "GDPR Art. 8: member states set the child-consent age between 13 and 16; detection uses 16 so every member-state boundary is covered.",
                    "gdpr",
                ),
                band_entry(
                    "UK",
                    13,
                    18,
                    "UK GDPR Art. 8 fixes child consent at 13; the ICO Children's Code scopes in services likely accessed by under-18s.",
                    "uk-childrens-code",
                ),
                band_entry(
                    "US",
                    13,
                    13,
                    "COPPA covers services directed to under-13s or with actual knowledge of collecting under-13 data.",
                    "coppa",
                ),
                band_entry(
                    "US-CA",
                    16,
                    16,
                    "CCPA/CPRA: sale/sharing of under-16 data requires opt-in (parental under 13, consumer 13–15).",
                    "ccpa",
                ),
            ],
        )
    });
    (measures, methods, bands)
}

/// All maintained required measures, across every supported jurisdiction.
pub fn measures() -> &'static [RequiredMeasure] {
    tables().0
}

/// All maintained certified methods of implementation.
pub fn methods() -> &'static [CertifiedMethod] {
    tables().1
}

/// The age bands (authorization + coverage ages) per supported jurisdiction.
pub fn bands() -> &'static [ConsentBand] {
    tables().2
}

/// Measures maintained for a jurisdiction code (canonicalized). Unknown
/// codes yield an empty slice — an explicit coverage gap, never an
/// inferred pass.
pub fn measures_for(jurisdiction: &str) -> Vec<&'static RequiredMeasure> {
    let code = canonical(jurisdiction);
    measures()
        .iter()
        .filter(|m| m.jurisdiction == code)
        .collect()
}

/// Certified methods maintained for a jurisdiction code (canonicalized).
pub fn methods_for(jurisdiction: &str) -> Vec<&'static CertifiedMethod> {
    let code = canonical(jurisdiction);
    methods()
        .iter()
        .filter(|m| m.jurisdiction == code)
        .collect()
}

/// Certified methods that implement the given required-measure id.
pub fn methods_implementing(measure_id: &str) -> Vec<&'static CertifiedMethod> {
    methods()
        .iter()
        .filter(|m| m.implements == measure_id)
        .collect()
}

/// Look up a single measure by id.
pub fn measure(id: &str) -> Option<&'static RequiredMeasure> {
    measures().iter().find(|m| m.id == id)
}

/// The consent band for a jurisdiction code (canonicalized), if maintained.
pub fn band(jurisdiction: &str) -> Option<&'static ConsentBand> {
    let code = canonical(jurisdiction);
    bands().iter().find(|b| b.jurisdiction == code)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionSection {
    pub jurisdiction: String,
    pub measures: Vec<RequiredMeasure>,
    pub certified_methods: Vec<CertifiedMethod>,
    pub consent_band: Option<ConsentBand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexReport {
    pub schema_version: String,
    pub rules_reviewed_on: String,
    pub limitations: Vec<String>,
    pub supported_jurisdictions: Vec<String>,
    pub jurisdictions: Vec<JurisdictionSection>,
}

/// The full indices, grouped per jurisdiction for rendering or JSON output.
pub fn index_report() -> IndexReport {
    IndexReport {
        schema_version: SCHEMA.into(),
        rules_reviewed_on: REVIEWED.into(),
        limitations: vec![
            "Dated offline reference set, not legal advice or a compliance certificate. Triggers match declared profile facts; they never measure actual behavior or verify implementation.".into(),
            "Only maintained jurisdictions carry index entries. An unsupported jurisdiction is a coverage gap requiring local review, not an inferred absence of obligations.".into(),
            "Certified methods are mechanisms an authority has approved, recognized or codified for the linked measure as of the reviewed date; availability, sector scope and commencement details must be rechecked before reliance.".into(),
            "Some measures have no certified method on record (for example EU/UK child consent, which requires reasonable-efforts verification instead). An empty method list is a statement about the index, not permission to skip the measure.".into(),
        ],
        supported_jurisdictions: SUPPORTED.iter().map(|s| (*s).into()).collect(),
        jurisdictions: SUPPORTED
            .iter()
            .map(|j| JurisdictionSection {
                jurisdiction: (*j).into(),
                measures: measures_for(j).into_iter().cloned().collect(),
                certified_methods: methods_for(j).into_iter().cloned().collect(),
                consent_band: band(j).cloned(),
            })
            .collect(),
    }
}

pub fn render(report: &IndexReport) -> String {
    let mut out = format!(
        "Jurisdiction index ({}) — reviewed {}\n",
        report.schema_version, report.rules_reviewed_on
    );
    for note in &report.limitations {
        out.push_str(&format!("Note: {note}\n"));
    }
    out.push_str(&format!(
        "Supported jurisdictions: {}\n",
        report.supported_jurisdictions.join(", ")
    ));
    for section in &report.jurisdictions {
        out.push_str(&format!("\n== {} ==\n", section.jurisdiction));
        match &section.consent_band {
            Some(b) => out.push_str(&format!(
                "Age band: parental authorization below {}; wider child-protection coverage below {}. {}\n",
                b.authorization_age, b.coverage_age, b.rule
            )),
            None => out.push_str("Age band: none maintained.\n"),
        }
        for m in &section.measures {
            out.push_str(&format!(
                "Required measure {} — {}\n  Obligation: {}\n  Legal basis: {}  Source: {}\n",
                m.id, m.name, m.obligation, m.legal_basis, m.source_id
            ));
            let linked: Vec<&CertifiedMethod> = methods_implementing(&m.id);
            if linked.is_empty() {
                out.push_str("  Certified methods: none maintained for this measure.\n");
            } else {
                for cm in linked {
                    out.push_str(&format!(
                        "  Certified method {} — {} ({}; {:?}; {})\n    {}\n",
                        cm.id, cm.name, cm.certifier, cm.status, cm.instrument, cm.reference_url
                    ));
                }
            }
        }
    }
    out
}

pub fn run(json: bool) -> anyhow::Result<()> {
    let report = index_report();
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", render(&report));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_measure_has_a_distinct_id_and_nonempty_fields() {
        let mut ids: Vec<&str> = measures().iter().map(|m| m.id.as_str()).collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "duplicate measure id");
        for m in measures() {
            assert!(SUPPORTED.contains(&m.jurisdiction.as_str()), "{}", m.id);
            assert!(!m.name.is_empty() && !m.obligation.is_empty(), "{}", m.id);
            assert!(
                !m.legal_basis.is_empty() && !m.source_id.is_empty(),
                "{}",
                m.id
            );
            assert_eq!(m.reviewed_on, REVIEWED, "{}", m.id);
        }
    }

    #[test]
    fn every_method_links_to_a_maintained_measure_in_its_jurisdiction() {
        for cm in methods() {
            let target = measure(&cm.implements).unwrap_or_else(|| {
                panic!("{} implements unknown measure {}", cm.id, cm.implements)
            });
            assert_eq!(
                target.jurisdiction, cm.jurisdiction,
                "{} links across jurisdictions",
                cm.id
            );
            assert!(SUPPORTED.contains(&cm.jurisdiction.as_str()), "{}", cm.id);
            assert!(
                !cm.certifier.is_empty() && !cm.instrument.is_empty(),
                "{}",
                cm.id
            );
            assert!(!cm.reference_url.is_empty(), "{}", cm.id);
        }
    }

    #[test]
    fn jurisdictional_lookups_are_canonicalized_and_exclusive() {
        assert_eq!(measures_for("eu").len(), 5);
        assert_eq!(measures_for("EEA").len(), 5);
        assert_eq!(measures_for("gb").len(), 3);
        assert!(measures_for("JP").is_empty());
        assert_eq!(band("uk").unwrap().authorization_age, 13);
        assert_eq!(band("EU").unwrap().authorization_age, 16);
        assert!(band("BR").is_none());
        for m in measures_for("US-CA") {
            assert_eq!(m.jurisdiction, "US-CA");
        }
    }

    #[test]
    fn trigger_evaluation_never_treats_unknown_as_true() {
        let facts = MeasureFacts::default();
        for m in measures() {
            assert!(m.applies(&facts).is_none(), "{} fired on empty facts", m.id);
        }
        let child = MeasureFacts {
            children: Some(true),
            ..Default::default()
        };
        assert!(measure("us-coppa-parental-consent")
            .unwrap()
            .applies(&child)
            .is_some());
        assert!(measure("eu-child-authorization")
            .unwrap()
            .applies(&child)
            .is_some());
        assert!(measure("uk-childrens-code")
            .unwrap()
            .applies(&child)
            .is_some());
        assert!(measure("eu-transfer-safeguards")
            .unwrap()
            .applies(&child)
            .is_none());
        let min12 = MeasureFacts {
            minimum_age: Some(12),
            ..Default::default()
        };
        assert!(measure("eu-child-authorization")
            .unwrap()
            .applies(&min12)
            .is_some());
        assert!(measure("uk-child-authorization")
            .unwrap()
            .applies(&min12)
            .is_some());
        let min13 = MeasureFacts {
            minimum_age: Some(13),
            ..Default::default()
        };
        assert!(measure("eu-child-authorization")
            .unwrap()
            .applies(&min13)
            .is_some());
        assert!(measure("uk-child-authorization")
            .unwrap()
            .applies(&min13)
            .is_none());
        assert!(measure("uk-childrens-code")
            .unwrap()
            .applies(&min13)
            .is_some());
        assert!(measure("us-coppa-parental-consent")
            .unwrap()
            .applies(&min13)
            .is_none());
    }

    #[test]
    fn transfer_and_service_triggers_follow_declared_facts() {
        let transfer = MeasureFacts {
            transfers: true,
            ..Default::default()
        };
        assert!(measure("eu-transfer-safeguards")
            .unwrap()
            .applies(&transfer)
            .is_some());
        assert!(measure("uk-restricted-transfer-route")
            .unwrap()
            .applies(&transfer)
            .is_some());
        let content_only = MeasureFacts {
            user_content: Some(true),
            ..Default::default()
        };
        assert!(measure("eu-notice-and-action")
            .unwrap()
            .applies(&content_only)
            .is_none());
        let hosted_content = MeasureFacts {
            user_content: Some(true),
            operates_service: true,
            ..Default::default()
        };
        assert!(measure("eu-notice-and-action")
            .unwrap()
            .applies(&hosted_content)
            .is_some());
    }

    #[test]
    fn index_report_covers_every_supported_jurisdiction() {
        let report = index_report();
        assert_eq!(report.jurisdictions.len(), SUPPORTED.len());
        for (section, code) in report.jurisdictions.iter().zip(SUPPORTED.iter()) {
            assert_eq!(&section.jurisdiction, code);
            assert!(!section.measures.is_empty(), "{code} lost its measures");
            assert!(section.consent_band.is_some(), "{code} lost its band");
        }
        let text = render(&report);
        assert!(text.contains("2021/914"));
        assert!(text.contains("312.11(c)"));
    }
}
