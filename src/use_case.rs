// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
//! Offline, declared-context risk triage. Recommendations are separate from
//! licensing evidence and never certify compliance or execute remedies.
use crate::jurisdiction::{self, canonical, ConsentBand, MeasureFacts};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

pub const SCHEMA: &str = "lwoodz.use-case-analysis/v1";
pub const REVIEWED: &str = "2026-10-03";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    Operate,
    Host,
    ShareSource,
    DistributeBinary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Market {
    Consumer,
    Enterprise,
    Education,
    Healthcare,
    Finance,
    Employment,
    DeveloperTools,
}

/// Age assurance mechanism at the service entry boundary, as declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgeGateKind {
    /// The user states their age or date of birth; nothing is verified.
    SelfDeclared,
    /// Independent age assurance: document/ID check, verified parental
    /// consent, or a trusted third-party age signal.
    Verified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub target: String,
    pub use_case: String,
    pub markets: Vec<Market>,
    pub activities: Vec<Activity>,
    /// EU/EEA, UK/GB, US, US-CA; other values remain explicit coverage gaps.
    #[serde(default)]
    pub user_jurisdictions: Vec<String>,
    #[serde(default)]
    pub developer_jurisdictions: Vec<String>,
    #[serde(default)]
    pub hosting_jurisdictions: Vec<String>,
    #[serde(default)]
    pub personal_data: Option<bool>,
    #[serde(default)]
    pub sensitive_data: Option<bool>,
    #[serde(default)]
    pub children: Option<bool>,
    /// Lowest age at which independent sign-up/use is permitted. Drives
    /// consent-band detection; unknown when omitted.
    #[serde(default)]
    pub minimum_age: Option<u8>,
    /// Age assurance declared at the entry boundary, if any.
    #[serde(default)]
    pub age_gate: Option<AgeGateKind>,
    #[serde(default)]
    pub automated_decisions: Option<bool>,
    #[serde(default)]
    pub user_content: Option<bool>,
    #[serde(default)]
    pub cross_border_transfers: Option<bool>,
    /// Selected/observed SPDX expressions; these declarations are not a scan.
    #[serde(default)]
    pub dependency_licenses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profiles {
    pub targets: Vec<Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub url: String,
    pub reviewed_on: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Remedy {
    pub timeframe: String,
    pub owner: String,
    pub action: String,
    pub evidence: String,
    pub residual_risk: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directive {
    pub id: String,
    pub target: String,
    pub inputs: Vec<String>,
    pub implementation: String,
    pub acceptance_tests: Vec<String>,
    pub evidence_artifact: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: String,
    pub priority: String,
    pub trigger: String,
    pub user_risk: String,
    pub developer_exposure: String,
    pub applicability: String,
    pub source_ids: Vec<String>,
    pub remedies: Vec<Remedy>,
    pub directive: Directive,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionReview {
    pub jurisdiction: String,
    pub connections: Vec<String>,
    pub coverage: String,
    pub variation: String,
    pub source_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketAnalysis {
    pub market: Market,
    pub adoption_requirements: String,
    pub recommended_entry: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgeBand {
    pub jurisdiction: String,
    /// Below this age a parent/guardian must authorize collection.
    pub authorization_age: u8,
    /// Below this age wider child-protection design rules scope in.
    pub coverage_age: u8,
    pub rule: String,
    pub source_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgeGateFinding {
    pub code: String,
    pub priority: String,
    pub message: String,
    pub remedy: String,
    pub jurisdictions: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgeGateAssessment {
    pub minimum_age: Option<u8>,
    pub gate: Option<AgeGateKind>,
    /// Consent bands maintained for the declared user jurisdictions.
    pub bands: Vec<AgeBand>,
    pub findings: Vec<AgeGateFinding>,
    /// True while any P0 finding is open.
    pub blocked: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicableMethod {
    pub id: String,
    pub name: String,
    pub certifier: String,
    pub instrument: String,
    pub status: String,
    pub reference_url: String,
    pub reviewed_on: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicableMeasure {
    pub id: String,
    pub jurisdiction: String,
    pub name: String,
    pub obligation: String,
    pub legal_basis: String,
    pub source_id: String,
    pub reviewed_on: String,
    /// Declared facts that made the measure apply.
    pub triggered_by: Vec<String>,
    /// Certified methods of implementation maintained for this measure. An
    /// empty list means none are on record, not that the measure is optional.
    pub certified_methods: Vec<ApplicableMethod>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetAnalysis {
    pub profile: Profile,
    pub market_analysis: Vec<MarketAnalysis>,
    pub unknowns: Vec<String>,
    pub jurisdiction_reviews: Vec<JurisdictionReview>,
    pub age_gate: AgeGateAssessment,
    pub required_measures: Vec<ApplicableMeasure>,
    pub recommendations: Vec<Recommendation>,
    pub release_review_required: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: String,
    pub rules_reviewed_on: String,
    pub limitations: Vec<String>,
    pub sources: Vec<Source>,
    pub targets: Vec<TargetAnalysis>,
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| (*s).into()).collect()
}
impl Profiles {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(!self.targets.is_empty(), "at least one target is required");
        let mut ids = BTreeSet::new();
        for p in &self.targets {
            anyhow::ensure!(
                !p.target.trim().is_empty() && !p.use_case.trim().is_empty(),
                "target and use_case must be nonempty"
            );
            anyhow::ensure!(
                ids.insert(p.target.trim()),
                "duplicate target: {}",
                p.target
            );
            anyhow::ensure!(
                !p.markets.is_empty() && !p.activities.is_empty(),
                "{} needs markets and activities",
                p.target
            );
            anyhow::ensure!(
                !(p.sensitive_data == Some(true) && p.personal_data == Some(false)),
                "{}: sensitive_data contradicts personal_data=false",
                p.target
            );
            for j in p
                .user_jurisdictions
                .iter()
                .chain(&p.developer_jurisdictions)
                .chain(&p.hosting_jurisdictions)
            {
                anyhow::ensure!(!j.trim().is_empty(), "jurisdiction must be nonempty");
            }
            for l in &p.dependency_licenses {
                anyhow::ensure!(!l.trim().is_empty(), "dependency license must be nonempty");
            }
        }
        Ok(())
    }
}

pub fn load(path: &Path) -> anyhow::Result<Profiles> {
    use anyhow::Context;
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read profile {}", path.display()))?;
    let profiles: Profiles = if path.extension().and_then(|s| s.to_str()) == Some("json") {
        serde_json::from_str(&text).context("invalid JSON use-case profile")?
    } else {
        toml::from_str(&text).context("invalid TOML use-case profile")?
    };
    profiles.validate()?;
    Ok(profiles)
}

pub fn analyze(profiles: &Profiles) -> anyhow::Result<Report> {
    profiles.validate()?;
    Ok(Report {
        schema_version: SCHEMA.into(), rules_reviewed_on: REVIEWED.into(),
        limitations: strings(&[
            "Declared-context triage, not legal advice, a legal determination, or a compliance certificate. No repository scan or control verification is performed.",
            "Priority reflects potential impact and urgency, not a measured probability or legal liability score. Remedies are proposed engineering actions, not statutory deadlines.",
            "Sources are a dated offline reference set. Recheck current law, commencement dates, local rules and contractual duties before release; unsupported jurisdictions require local review.",
            "Age-gate and required-measure detection evaluate declared fields against the dated per-jurisdiction index (lwoodz.jurisdiction-index/v1); they do not verify a deployed gate, a signed instrument or the current legal text.",
            "No application changes, legal documents, external requests or executable compliance tools are produced. Directives specify tools to implement and verify for each target.",
        ]),
        sources: [
            ("gdpr", "https://eur-lex.europa.eu/eli/reg/2016/679/oj/eng"),
            ("uk-transfer", "https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/international-transfers/a-guide-to-international-transfers/"),
            ("uk-gdpr", "https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/"),
            ("uk-childrens-code", "https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/childrens-code-guidance-and-resources/"),
            ("coppa", "https://www.ftc.gov/legal-library/browse/rules/childrens-online-privacy-protection-rule-coppa"),
            ("ccpa", "https://oag.ca.gov/privacy/ccpa"),
            ("hipaa", "https://www.hhs.gov/hipaa/for-professionals/covered-entities/index.html"),
            ("eu-ai", "https://digital-strategy.ec.europa.eu/en/policies/regulatory-framework-ai"),
            ("agpl", "https://www.gnu.org/licenses/agpl-3.0.html"),
            ("dsa", "https://digital-strategy.ec.europa.eu/en/policies/digital-services-act-package"),
        ].into_iter().map(|(id,url)| Source { id: id.into(), url: url.into(), reviewed_on: REVIEWED.into() }).collect(),
        targets: profiles.targets.iter().map(analyze_target).collect(),
    })
}

struct Rule<'a> {
    id: &'a str,
    priority: &'a str,
    trigger: &'a str,
    user: &'a str,
    developer: &'a str,
    applicability: &'a str,
    sources: &'a [&'a str],
    immediate: &'a str,
    implement: &'a str,
    tests: &'a [&'a str],
}
fn recommendation(p: &Profile, r: Rule<'_>) -> Recommendation {
    Recommendation {
        id: r.id.into(), priority: r.priority.into(), trigger: r.trigger.into(),
        user_risk: r.user.into(), developer_exposure: r.developer.into(),
        applicability: r.applicability.into(), source_ids: strings(r.sources),
        remedies: vec![Remedy {
            timeframe: "Before launch, or within 48 hours for an exposed live feature (planning target)".into(),
            owner: "Product owner and operations lead".into(), action: r.immediate.into(),
            evidence: format!("{}: approved change record and verification results", r.id),
            residual_risk: "Temporary containment does not resolve past exposure, legal applicability or all obligations; obtain qualified review before re-enabling the affected workflow.".into(),
        }, Remedy {
            timeframe: "Next 7–30 days (planning target)".into(), owner: "Engineering and legal reviewer".into(),
            action: format!("Implement {} for {}; verify applicable law, roles, contracts and control effectiveness.", r.id, p.target),
            evidence: format!("{}: test results, reviewer decision and policy version", r.id),
            residual_risk: "Passing technical checks is evidence of specific controls, not proof of legal compliance.".into(),
        }],
        directive: Directive {
            id: format!("{}:control", r.id), target: p.target.clone(),
            inputs: vec![format!("Use case: {}", p.use_case), format!("Activities: {:?}; markets: {:?}", p.activities,p.markets),
                "Versioned jurisdiction/role policy approved by a reviewer; inventory of data flows, vendors and affected interfaces".into()],
            implementation: r.implement.into(), acceptance_tests: strings(r.tests),
            evidence_artifact: format!("{}: control-result JSON with target, rule ID, policy version, timestamp, pass/fail/unknown, and redacted evidence references", r.id),
        },
    }
}

/// Age gate detection: compare the declared `children` / `minimum_age` /
/// `age_gate` facts against the consent bands maintained for the declared
/// user jurisdictions. Omitted facts are unknown, never false, so detection
/// reports scope gaps instead of silently passing them.
fn detect_age_gate(p: &Profile, user_jurisdictions: &BTreeSet<String>) -> AgeGateAssessment {
    let mut bands: Vec<AgeBand> = Vec::new();
    for j in user_jurisdictions {
        if let Some(b) = jurisdiction::band(j) {
            bands.push(to_age_band(b));
        }
    }
    let band_jurisdictions: Vec<String> = bands.iter().map(|b| b.jurisdiction.clone()).collect();
    let below_any = |floor: u8| p.minimum_age.map(|m| m < floor).unwrap_or(false);
    let severity = if p.children == Some(true) { "P0" } else { "P1" };
    let mut findings = Vec::new();
    let mut finding = |code: &str,
                       priority: &str,
                       message: String,
                       remedy: String,
                       jurisdictions: Vec<String>| {
        findings.push(AgeGateFinding {
            code: code.into(),
            priority: priority.into(),
            message,
            remedy,
            jurisdictions,
        });
    };
    if p.children.is_none() && p.minimum_age.is_none() {
        finding(
            "AGE_SCOPE_UNKNOWN",
            "P1",
            "Neither children nor minimum_age is declared; age-related obligations cannot be scoped and consent-band checks are skipped."
                .into(),
            "Declare children and minimum_age (an explicit value counts) before relying on age-gate conclusions."
                .into(),
            band_jurisdictions.clone(),
        );
    }
    if p.children == Some(true) && p.minimum_age.is_none() && p.age_gate.is_none() {
        finding(
            "AGE_GATE_MISSING",
            "P1",
            "Children are declared in the audience but neither an age gate nor a minimum_age floor is declared."
                .into(),
            "Declare the entry control (age_gate kind and minimum_age) and enforce it server-side before any collection."
                .into(),
            band_jurisdictions.clone(),
        );
    }
    if p.age_gate.is_some() && p.minimum_age.is_none() {
        finding(
            "AGE_GATE_WITHOUT_FLOOR",
            "P1",
            "An age gate is declared without a minimum_age floor, so band adequacy cannot be checked."
                .into(),
            "Declare minimum_age and verify the gate blocks sign-up below each applicable jurisdiction band."
                .into(),
            band_jurisdictions.clone(),
        );
    }
    for b in &bands {
        if below_any(b.authorization_age) {
            let coppa = b.jurisdiction == "US";
            finding(
                "BELOW_AUTHORIZATION_BAND",
                severity,
                format!(
                    "minimum_age {} is below the {} parental-authorization age of {}{}.",
                    p.minimum_age.unwrap_or_default(),
                    b.jurisdiction,
                    b.authorization_age,
                    if coppa { " (COPPA verifiable consent)" } else { "" },
                ),
                format!(
                    "Raise minimum_age to {}+ or implement {}authorization with verifiable parental consent and revocation propagation.",
                    b.authorization_age,
                    if coppa { "COPPA-compliant " } else { "" },
                ),
                vec![b.jurisdiction.clone()],
            );
        }
        if b.coverage_age > b.authorization_age
            && (p.children == Some(true) || below_any(b.coverage_age))
        {
            finding(
                "CHILD_PROTECTION_CODE_SCOPE",
                "P1",
                format!(
                    "Service is in scope of {} child-protection design rules covering under-{}s{}.",
                    b.jurisdiction,
                    b.coverage_age,
                    if p.children == Some(true) {
                        " (children declared in the audience)"
                    } else {
                        " via the declared minimum_age"
                    },
                ),
                format!(
                    "Apply the {} design standards: age-appropriate defaults, data minimization, profiling off by default, and guardian controls."
                        ,
                    b.jurisdiction
                ),
                vec![b.jurisdiction.clone()],
            );
        }
    }
    if p.age_gate == Some(AgeGateKind::SelfDeclared)
        && (p.children == Some(true)
            || p.sensitive_data == Some(true)
            || bands.iter().any(|b| below_any(b.authorization_age)))
    {
        finding(
            "SELF_DECLARED_GATE_INSUFFICIENT",
            "P1",
            "Age gate is self-declared only while verifiable authorization or sensitive-data duties may apply; attested age is unverified."
                .into(),
            "Use verified parental consent or an independent age-assurance signal where a band requires it; keep the self-declared path only as a first filter."
                .into(),
            band_jurisdictions.clone(),
        );
    }
    let blocked = findings.iter().any(|f| f.priority == "P0");
    AgeGateAssessment {
        minimum_age: p.minimum_age,
        gate: p.age_gate,
        bands,
        findings,
        blocked,
    }
}

fn to_age_band(b: &ConsentBand) -> AgeBand {
    AgeBand {
        jurisdiction: b.jurisdiction.clone(),
        authorization_age: b.authorization_age,
        coverage_age: b.coverage_age,
        rule: b.rule.clone(),
        source_ids: vec![b.source_id.clone()],
    }
}

/// Requirement detection: evaluate the per-jurisdiction index of required
/// measures against the declared facts, attaching the certified methods of
/// implementation maintained for each measure that applies.
fn detect_requirements(
    jurisdictions: &BTreeSet<String>,
    facts: &MeasureFacts,
) -> Vec<ApplicableMeasure> {
    let mut out = Vec::new();
    for j in jurisdictions {
        for m in jurisdiction::measures_for(j) {
            if let Some(triggered_by) = m.applies(facts) {
                out.push(ApplicableMeasure {
                    id: m.id.clone(),
                    jurisdiction: m.jurisdiction.clone(),
                    name: m.name.clone(),
                    obligation: m.obligation.clone(),
                    legal_basis: m.legal_basis.clone(),
                    source_id: m.source_id.clone(),
                    reviewed_on: m.reviewed_on.clone(),
                    triggered_by,
                    certified_methods: jurisdiction::methods_implementing(&m.id)
                        .into_iter()
                        .map(|cm| ApplicableMethod {
                            id: cm.id.clone(),
                            name: cm.name.clone(),
                            certifier: cm.certifier.clone(),
                            instrument: cm.instrument.clone(),
                            status: cm.status.label().to_string(),
                            reference_url: cm.reference_url.clone(),
                            reviewed_on: cm.reviewed_on.clone(),
                        })
                        .collect(),
                });
            }
        }
    }
    out.sort_by(|a, b| a.jurisdiction.cmp(&b.jurisdiction).then(a.id.cmp(&b.id)));
    out
}

fn analyze_target(p: &Profile) -> TargetAnalysis {
    let mut unknowns = Vec::new();
    for (name, value) in [
        ("personal_data", p.personal_data),
        ("sensitive_data", p.sensitive_data),
        ("children", p.children),
        ("automated_decisions", p.automated_decisions),
        ("user_content", p.user_content),
        ("cross_border_transfers", p.cross_border_transfers),
    ] {
        if value.is_none() {
            unknowns.push(format!(
                "Confirm {name}; omitted values are unknown, not false."
            ));
        }
    }
    for (name, values) in [
        ("user_jurisdictions", &p.user_jurisdictions),
        ("developer_jurisdictions", &p.developer_jurisdictions),
    ] {
        if values.is_empty() {
            unknowns.push(format!(
                "Supply {name}; market reach and establishment both matter."
            ));
        }
    }
    if p.activities.contains(&Activity::Host) && p.hosting_jurisdictions.is_empty() {
        unknowns.push(
            "Supply hosting_jurisdictions, including vendors, backups and support access.".into(),
        );
    }
    if p.children == Some(true) {
        if p.minimum_age.is_none() {
            unknowns.push(
                "Confirm minimum_age for the child audience; omitted values are unknown, not false."
                    .into(),
            );
        }
        if p.age_gate.is_none() {
            unknowns.push(
                "Confirm age_gate (self_declared or verified); omitted values are unknown, not false."
                    .into(),
            );
        }
    }
    if p.dependency_licenses.is_empty() {
        unknowns.push("Dependency licenses are unassessed; provide expressions and run the separate licensing audit.".into());
    }
    let user_jurisdictions: BTreeSet<String> =
        p.user_jurisdictions.iter().map(|s| canonical(s)).collect();
    let jurisdictions: BTreeSet<String> = p
        .user_jurisdictions
        .iter()
        .chain(&p.developer_jurisdictions)
        .chain(&p.hosting_jurisdictions)
        .map(|s| canonical(s))
        .collect();
    let transfer_signal = p.cross_border_transfers == Some(true)
        || (p.personal_data != Some(false)
            && p.user_jurisdictions.iter().any(|u| {
                p.hosting_jurisdictions
                    .iter()
                    .any(|h| canonical(u) != canonical(h))
            }));
    let facts = MeasureFacts {
        children: p.children,
        minimum_age: p.minimum_age,
        personal_data: p.personal_data,
        sensitive_data: p.sensitive_data,
        automated_decisions: p.automated_decisions,
        user_content: p.user_content,
        transfers: transfer_signal,
        operates_service: p.activities.contains(&Activity::Operate)
            || p.activities.contains(&Activity::Host),
    };
    let age_gate = detect_age_gate(p, &user_jurisdictions);
    let required_measures = detect_requirements(&jurisdictions, &facts);
    let has = |j: &str| jurisdictions.contains(j);
    let us = has("US") || has("US-CA");
    let mut reviews = Vec::new();
    for j in &jurisdictions {
        let (coverage, variation, sources) = match j.as_str() {
            "EU" => ("partial", "GDPR territorial scope depends on establishment, offering services or monitoring, not server location alone. National rules can vary, including consent ages. Assess roles, lawful basis and transfer safeguards.", vec!["gdpr"]),
            "UK" => ("partial", "Review UK GDPR and current UK amendments separately. Restricted transfers require a UK-specific assessment and a valid route; an EU mechanism alone is not automatic UK coverage.", vec!["uk-transfer"]),
            "US" => ("partial", "Federal sector rules and state privacy laws vary. Identify actual states, thresholds and exemptions. COPPA concerns covered services directed to under-13s or with actual knowledge; it is not a rule for all minors.", vec!["coppa","ccpa","hipaa"]),
            "US-CA" => ("partial", "CCPA coverage depends on business thresholds and exemptions; California access alone does not establish applicability. Evaluate sale/sharing and consumer-rights controls for covered activity.", vec!["ccpa"]),
            _ => { unknowns.push(format!("No legal rules maintained for {j}; obtain local review, including sector, privacy, hosting and distribution requirements.")); ("unsupported", "No jurisdiction-specific conclusion available. Do not infer approval from missing rules.", vec![]) }
        };
        let connections = [
            ("users", &p.user_jurisdictions),
            ("developer", &p.developer_jurisdictions),
            ("hosting", &p.hosting_jurisdictions),
        ]
        .into_iter()
        .filter(|(_, values)| values.iter().any(|v| canonical(v) == *j))
        .map(|(role, _)| role.to_string())
        .collect();
        reviews.push(JurisdictionReview {
            jurisdiction: j.clone(),
            connections,
            coverage: coverage.into(),
            variation: variation.into(),
            source_ids: strings(&sources),
        });
    }
    let mut recs = Vec::new();
    let mut add = |r| recs.push(recommendation(p, r));
    if !unknowns.is_empty() {
        add(Rule {
        id:"SCOPE_REVIEW", priority:"P1", trigger:"Missing context or unsupported jurisdiction",
        user:"Unidentified users and data flows can conceal harmful exposure.",developer:"Incomplete facts prevent reliable assessment of operational duties.",
        applicability:"Applicability unresolved; these are discovery tasks.",sources:&[],
        immediate:"Inventory users, territories, operator roles, vendors and data; restrict an unassessed launch to a controlled pilot while review proceeds.",
        implement:"Build a profile completeness gate. Require explicit answers and per-jurisdiction review records; return unknown for missing or stale policies, never pass.",
        tests:&["Omitted flags produce unknown; explicit false is preserved.","Unsupported territory requires review; adding a territory invalidates prior sign-off."] });
    }
    if p.activities.contains(&Activity::Operate) {
        add(Rule {
        id:"OPERATION",priority:"P1",trigger:"Developer operates the application",user:"Service failure, misleading claims or ineffective redress can cause loss.",
        developer:"Operator conduct, product claims, security failures and contracts can create exposure beyond source licensing.",applicability:"Review consumer, sector and contractual duties for each market; operation alone is not a legal violation.",sources:&[],
        immediate:"Assign an incident owner, check public claims against tested behavior, and provide a working complaint and rollback route.",
        implement:"Build a release gate mapping each product claim and critical workflow to evidence, an accountable owner, incident routing and rollback controls.",
        tests:&["Untested claims block sign-off.","Exercise incident escalation, complaint handling and rollback with synthetic data."] });
    }
    if p.activities.contains(&Activity::Host) {
        add(Rule {
        id:"HOSTING",priority:"P1",trigger:"Developer hosts the application",user:"Tenant leakage and vendor access may expose user information.",developer:"Hosting creates operational and vendor-management exposure; outsourcing infrastructure does not decide responsibility.",
        applicability:"Determine actual controller/processor or other service roles from facts and contracts.",sources: if has("EU") { &["gdpr"] } else { &[] },
        immediate:"Restrict admin access, disable public storage, inventory subprocessors and verify restore/incident contacts.",
        implement:"Build tenant-isolation and infrastructure-policy checks covering storage ACLs, secret handling, backup location, support access and vendor approval expiry.",
        tests:&["Cross-tenant reads and public object access fail.","Unapproved vendor or backup region blocks deployment; logs contain no payload secrets."] });
    }
    if p.personal_data == Some(true) || p.sensitive_data == Some(true) {
        add(Rule {
        id:"DATA_GOVERNANCE",priority:if p.sensitive_data==Some(true) {"P0"} else {"P1"},trigger:"Personal or sensitive data declared",user:"Disclosure, profiling or excessive retention can harm privacy and safety.",developer:"Processing may create privacy, security, rights-handling and breach-response duties.",
        applicability:"Verify scope and lawful processing conditions; sensitive data may need additional conditions. Consent is not universally the correct basis.",sources:if has("EU") { &["gdpr"] } else if has("US-CA") { &["ccpa"] } else { &[] },
        immediate:"Stop unnecessary collection, redact telemetry and narrow access; record required retention and legal holds before deleting data.",
        implement:"Build a purpose/field inventory, configurable retention runner and identity-verified rights workflow across primary stores, vendors and backups. Enforce policy by tenant and territory; preserve documented legal holds.",
        tests:&["Deletion removes eligible records across stores while preserving documented holds.","Rights requests cannot reveal another user's data; expired retention produces evidence."] });
    }
    if p.children == Some(true) {
        add(Rule {
        id:"CHILDREN",priority:"P0",trigger:"Children are in the declared audience",user:"Children may disclose data without understanding the consequences or encounter inappropriate content.",developer:"Age-specific privacy and product-design obligations may apply; school or parental involvement does not automatically remove operator duties.",
        applicability:"Confirm age bands and service audience. For US coverage, assess under-13 targeting or actual knowledge; EU consent ages vary where the relevant consent rule applies.",sources:if us { &["coppa"] } else if has("EU") { &["gdpr"] } else { &[] },
        immediate:"Disable unnecessary child profiling and risky collection; verify the applicable authorization workflow before opening affected features.",
        implement:"Build a territory/age-policy engine with minimized age signals, verifiable authorization where required, revocation propagation and accessible guardian notices. Avoid collecting identity documents by default.",
        tests:&["Missing or revoked required authorization blocks collection server-side.","Age boundaries follow the selected reviewed policy; UI bypass cannot enable restricted collection."] });
    }
    if !age_gate.findings.is_empty() {
        add(Rule {
        id:"AGE_GATE",priority:if age_gate.blocked {"P0"} else {"P1"},trigger:"Age gate or consent-band finding detected",
        user:"Under-age users may pass a weak gate and encounter collection or content their jurisdiction restricts.",developer:"Consent-band breaches create per-jurisdiction exposure that compounds with child-audience and data-governance duties.",
        applicability:"Findings reference declared minimum_age, age_gate and user jurisdictions only; they do not measure a deployed control.",sources:&[],
        immediate:"Read the age_gate section, close P0 findings before launch, and restrict the affected flow to a reviewed pilot meanwhile.",
        implement:"Build the declared age gate as an enforced server-side check: minimum_age floor, verified authorization below each applicable band, guardian-consent revocation propagation and evidence logging.",
        tests:&["A sign-up below minimum_age is rejected server-side, not only in the UI.","Band changes require policy review; bypass attempts produce audit evidence."] });
    }
    if p.automated_decisions == Some(true) {
        add(Rule {
        id:"AUTOMATED_DECISIONS",priority:"P0",trigger:"Automated decisions declared",user:"Incorrect or discriminatory decisions may deny services, jobs or essential opportunities.",developer:"Deployment role, intended use and decision effects can trigger privacy, equality or AI-specific duties.",
        applicability:"Classify intended use and operator/provider role; verify current AI Act commencement and exceptions. Not all AI is high-risk or subject to the same controls.",sources:if has("EU") { &["eu-ai","gdpr"] } else { &[] },
        immediate:"Require meaningful human review for consequential decisions, add appeal routing and suspend unsupported automated actions.",
        implement:"Build a decision register recording model/policy versions, intended use, test evidence and human overrides. Add representative error/bias evaluations, drift thresholds and an appeal workflow with minimal sensitive logging.",
        tests:&["Consequential actions cannot bypass required human approval.","Model/policy changes invalidate sign-off; appeals and rollback work end-to-end."] });
    }
    if transfer_signal {
        add(Rule {
        id:"TRANSFERS",priority:"P1",trigger:"Declared transfer or differing user/hosting territories suggest transfer review",user:"Foreign access may change privacy protections and available remedies.",developer:"Transfer restrictions may cover vendors and remote access as well as storage location.",applicability:"Different locations are a review signal, not proof of a restricted transfer. Determine exporter, importer, access, applicable law and legal route.",sources:if has("UK") { &["uk-transfer"] } else if has("EU") { &["gdpr"] } else { &[] },
        immediate:"Inventory foreign recipients and remote support; pause unreviewed personal-data routes while preserving required service and incident evidence.",
        implement:"Build a data-flow policy gate mapping recipients and access regions to approved transfer routes, contracts, expiry and reviewer decisions. Separate EU and UK policy versions.",
        tests:&["Unapproved destination or expired approval returns fail/unknown.","Backups, telemetry and remote support are evaluated as well as primary hosting."] });
    }
    if p.user_content == Some(true) {
        add(Rule {
        id:"CONTENT",priority:"P1",trigger:"User content declared",user:"Abusive, unlawful or privacy-invasive uploads can harm users and third parties.",developer:"Hosting/sharing content may create notice-handling, IP and platform obligations.",applicability:"Classify service function, territory and exemptions; not every application is an online platform.",sources:if has("EU") { &["dsa"] } else { &[] },
        immediate:"Provide an abuse-report channel and an accountable reviewer; restrict public uploads if no safe handling process exists.",
        implement:"Build notice intake, evidence-preserving review, proportionate access restriction and appeal workflows. Configure timing and reasons by reviewed jurisdiction policy; apply limited retention and reviewer access.",
        tests:&["A notice reaches the assigned owner and produces a reasoned decision.","Appeals can reverse errors; reporters cannot access private case evidence."] });
    }
    if p.activities.contains(&Activity::ShareSource)
        || p.activities.contains(&Activity::DistributeBinary)
    {
        add(Rule {
        id:"DISTRIBUTION",priority:"P1",trigger:"Source sharing or binary distribution declared",user:"Recipients may receive secrets, unauthorized material or incomplete license information.",developer:"Distribution can trigger component-specific notice, source and permission requirements; a root license does not settle every component.",applicability:"Review exact licenses, selected alternatives, modifications and distribution relationship; keyword matches are not compatibility decisions.",sources:&[],
        immediate:"Inspect the exact release artifact for secrets, permissions, third-party notices and required source before sharing.",
        implement:"Build an artifact-to-component inventory linking hashes to license expressions, origin and reviewed obligations; check required notices/source availability and secret scans in the release gate.",
        tests:&["An unreviewed component or missing required notice blocks release.","Validate the shipped archive/container, not only the source checkout."] });
    }
    if p.activities.contains(&Activity::Host)
        && p.dependency_licenses.iter().any(|l| {
            crate::license::spdx::SpdxExpression::parse(l.clone())
                .identifiers()
                .iter()
                .any(|id| id.starts_with("AGPL-"))
        })
    {
        add(Rule {
        id:"NETWORK_COPYLEFT",priority:"P1",trigger:"Hosting with a declared AGPL expression",user:"Network users may be unable to obtain source to which the applicable license entitles them.",developer:"Modified AGPL-covered programs can carry network source-offer obligations.",applicability:"Review the selected expression branch, modifications and covered work. An AGPL dependency does not automatically require disclosure of the whole application.",sources:&["agpl"],
        immediate:"Identify the covered version and modifications; review the source offer before expanding network access.",
        implement:"Build an AGPL review record and source-offer verifier tied to the deployed covered artifact and its build inputs; require a reviewer for ambiguous license alternatives.",
        tests:&["A required source offer resolves to the corresponding deployed version.","Ambiguous OR branches return review-required rather than automatic violation."] });
    }
    if p.markets.contains(&Market::Healthcare)
        || p.markets.contains(&Market::Finance)
        || p.markets.contains(&Market::Employment)
    {
        add(Rule {
        id:"SECTOR_REVIEW",priority:"P0",trigger:"Healthcare, finance or employment target market",user:"Errors or disclosure may affect health, money or access to employment.",developer:"Sector duties depend on function, relationships and claims; professional, product and discrimination rules may require specialist review.",applicability:"Health data alone does not establish HIPAA coverage: assess covered-entity/business-associate relationships. Financial and employment requirements need local functional review.",sources:if us && p.markets.contains(&Market::Healthcare) { &["hipaa"] } else { &[] },
        immediate:"Limit the pilot to reviewed workflows, remove unsupported professional claims and require accountable human escalation.",
        implement:"Build a sector/use-function approval registry linking intended purpose, counterparty role, claims, required agreements and evidence; prevent unreviewed expansion into consequential workflows.",
        tests:&["New clinical, financial or hiring use requires fresh review.","Missing required agreement or reviewer approval blocks affected workflow activation."] });
    }
    recs.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.id.cmp(&b.id)));
    let market_analysis = p.markets.iter().map(|m| {
        let (needs,entry) = match m {
            Market::Consumer => ("Clear claims, accessible support, privacy choices and predictable service behavior", "Pilot with limited collection and verified complaint handling before broad promotion"),
            Market::Enterprise => ("Buyer security evidence, contractual roles, tenant isolation and vendor transparency", "Offer a scoped tenant pilot with agreed data flows and procurement evidence"),
            Market::Education => ("Age bands, school/operator responsibilities and accessible guardian workflows", "Validate student/teacher flows and authorization boundaries with a controlled school pilot"),
            Market::Healthcare => ("Clinical purpose, health-data safeguards and counterparty-role review", "Start with a reviewed non-consequential workflow; validate claims before clinical expansion"),
            Market::Finance => ("Financial function, licensing perimeter, accuracy and redress review", "Use synthetic-data evaluation before enabling real-money or consequential advice workflows"),
            Market::Employment => ("Job-impact analysis, discrimination testing and meaningful appeal", "Pilot decision support with accountable human review before consequential automation"),
            Market::DeveloperTools => ("Artifact provenance, license notices, secure defaults and telemetry transparency", "Ship a reproducible scoped release with explicit telemetry and hosting boundaries"),
        };
        MarketAnalysis {market:*m,adoption_requirements:needs.into(),recommended_entry:entry.into()}
    }).collect();
    TargetAnalysis {
        profile: p.clone(),
        market_analysis,
        unknowns,
        jurisdiction_reviews: reviews,
        age_gate,
        required_measures,
        recommendations: recs,
        release_review_required: true,
    }
}

pub fn render(report: &Report) -> String {
    let mut out = format!(
        "Use-case analysis ({}) — rules reviewed {}\n",
        report.schema_version, report.rules_reviewed_on
    );
    for note in &report.limitations {
        out.push_str(&format!("Note: {note}\n"));
    }
    for t in &report.targets {
        out.push_str(&format!(
            "\nTarget: {} — {}\nRelease review required\n",
            t.profile.target, t.profile.use_case
        ));
        for m in &t.market_analysis {
            out.push_str(&format!(
                "Market {:?}: {}. Entry: {}.\n",
                m.market, m.adoption_requirements, m.recommended_entry
            ));
        }
        for u in &t.unknowns {
            out.push_str(&format!("Unknown: {u}\n"));
        }
        for j in &t.jurisdiction_reviews {
            out.push_str(&format!(
                "Jurisdiction {} ({}, {:?}): {} Sources: {}\n",
                j.jurisdiction,
                j.coverage,
                j.connections,
                j.variation,
                j.source_ids.join(", ")
            ));
        }
        out.push_str(&format!(
            "\nAge gate: minimum_age {}, gate {} — {}\n",
            t.age_gate
                .minimum_age
                .map(|m| m.to_string())
                .unwrap_or_else(|| "undeclared".into()),
            t.age_gate
                .gate
                .map(|g| format!("{g:?}"))
                .unwrap_or_else(|| "undeclared".into()),
            if t.age_gate.blocked {
                "BLOCKED (P0 finding open)"
            } else if t.age_gate.findings.is_empty() {
                "no findings"
            } else {
                "findings open"
            }
        ));
        for b in &t.age_gate.bands {
            out.push_str(&format!(
                "Age band {}: parental authorization below {}; wider coverage below {}. {} Sources: {}\n",
                b.jurisdiction,
                b.authorization_age,
                b.coverage_age,
                b.rule,
                b.source_ids.join(", ")
            ));
        }
        for f in &t.age_gate.findings {
            out.push_str(&format!(
                "Age finding [{}] {}: {}\n  Remedy: {}\n",
                f.priority, f.code, f.message, f.remedy
            ));
        }
        if t.required_measures.is_empty() {
            out.push_str("Required measures: none triggered by declared facts.\n");
        } else {
            out.push_str("Required measures:\n");
            for m in &t.required_measures {
                out.push_str(&format!(
                    "  [{}] {} — {}\n    Obligation: {}\n    Legal basis: {}  Source: {} (reviewed {})\n    Triggered by: {}\n",
                    m.jurisdiction,
                    m.id,
                    m.name,
                    m.obligation,
                    m.legal_basis,
                    m.source_id,
                    m.reviewed_on,
                    m.triggered_by.join("; ")
                ));
                if m.certified_methods.is_empty() {
                    out.push_str(
                        "    Certified methods: none maintained; implement directly and record verification.\n",
                    );
                } else {
                    for cm in &m.certified_methods {
                        out.push_str(&format!(
                            "    Certified method {} — {} ({}; {}; {})\n      {}\n",
                            cm.id,
                            cm.name,
                            cm.certifier,
                            cm.status,
                            cm.instrument,
                            cm.reference_url
                        ));
                    }
                }
            }
        }
        for r in &t.recommendations {
            out.push_str(&format!("\n[{}] {}\nTrigger: {}\nUser risk: {}\nDeveloper exposure: {}\nApplicability: {}\nSources: {}\n",r.priority,r.id,r.trigger,r.user_risk,r.developer_exposure,r.applicability,r.source_ids.join(", ")));
            for fix in &r.remedies {
                out.push_str(&format!(
                    "Remedy ({}; {}): {}\nEvidence: {}\nResidual risk: {}\n",
                    fix.timeframe, fix.owner, fix.action, fix.evidence, fix.residual_risk
                ));
            }
            out.push_str(&format!(
                "Tool {} for {}\nInputs: {}\nBuild: {}\n",
                r.directive.id,
                r.directive.target,
                r.directive.inputs.join("; "),
                r.directive.implementation
            ));
            for test in &r.directive.acceptance_tests {
                out.push_str(&format!("Acceptance: {test}\n"));
            }
            out.push_str(&format!("Artifact: {}\n", r.directive.evidence_artifact));
        }
    }
    out.push_str("\nReferences:\n");
    for s in &report.sources {
        out.push_str(&format!(
            "{}: {} (reviewed {})\n",
            s.id, s.url, s.reviewed_on
        ));
    }
    out
}

pub fn run(path: &Path, json: bool) -> anyhow::Result<()> {
    let report = analyze(&load(path)?)?;
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

    fn profile() -> Profile {
        toml::from_str(
            r#"
target = "t"
use_case = "u"
markets = ["consumer"]
activities = ["operate", "host"]
user_jurisdictions = ["EU", "UK", "US-CA"]
personal_data = true
children = true
minimum_age = 13
age_gate = "self_declared"
dependency_licenses = ["MIT"]
"#,
        )
        .unwrap()
    }

    #[test]
    fn age_gate_detection_reports_band_scope_and_gate_strength() {
        let p = profile();
        let users: BTreeSet<String> = p.user_jurisdictions.iter().map(|s| canonical(s)).collect();
        let a = detect_age_gate(&p, &users);
        assert!(a.blocked, "children=true makes band breaches P0");
        let codes: Vec<&str> = a.findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"BELOW_AUTHORIZATION_BAND"));
        assert!(codes.contains(&"CHILD_PROTECTION_CODE_SCOPE"));
        assert!(codes.contains(&"SELF_DECLARED_GATE_INSUFFICIENT"));
        assert!(!codes.contains(&"AGE_GATE_MISSING"));
        assert_eq!(a.bands.len(), 3);
        let uk = a.bands.iter().find(|b| b.jurisdiction == "UK").unwrap();
        assert_eq!(uk.coverage_age, 18);
        let eu = a
            .findings
            .iter()
            .find(|f| f.code == "BELOW_AUTHORIZATION_BAND" && f.jurisdictions == ["EU"])
            .unwrap();
        assert_eq!(eu.priority, "P0");
    }

    #[test]
    fn age_gate_detection_passes_clean_declarations() {
        let mut p = profile();
        p.children = Some(false);
        p.minimum_age = Some(18);
        p.age_gate = Some(AgeGateKind::Verified);
        p.sensitive_data = Some(false);
        let users: BTreeSet<String> = p.user_jurisdictions.iter().map(|s| canonical(s)).collect();
        let a = detect_age_gate(&p, &users);
        assert!(a.findings.is_empty(), "findings: {:?}", a.findings);
        assert!(!a.blocked);
    }

    #[test]
    fn age_gate_detection_flags_undeclared_scope_and_floorless_gates() {
        let mut p = profile();
        p.children = None;
        p.minimum_age = None;
        p.age_gate = None;
        let users: BTreeSet<String> = p.user_jurisdictions.iter().map(|s| canonical(s)).collect();
        let a = detect_age_gate(&p, &users);
        let codes: Vec<&str> = a.findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"AGE_SCOPE_UNKNOWN"));
        assert!(!codes.contains(&"BELOW_AUTHORIZATION_BAND"));

        let mut p = profile();
        p.minimum_age = None;
        let a = detect_age_gate(&p, &users);
        let codes: Vec<&str> = a.findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"AGE_GATE_WITHOUT_FLOOR"));
        assert!(!codes.contains(&"AGE_GATE_MISSING"));

        let mut p = profile();
        p.minimum_age = None;
        p.age_gate = None;
        let a = detect_age_gate(&p, &users);
        let codes: Vec<&str> = a.findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"AGE_GATE_MISSING"));
        assert!(!codes.contains(&"AGE_GATE_WITHOUT_FLOOR"));
    }

    #[test]
    fn requirement_detection_links_measures_to_certified_methods() {
        let p = profile();
        let jurisdictions: BTreeSet<String> = p
            .user_jurisdictions
            .iter()
            .chain(&p.developer_jurisdictions)
            .chain(&p.hosting_jurisdictions)
            .map(|s| canonical(s))
            .collect();
        let facts = MeasureFacts {
            children: p.children,
            minimum_age: p.minimum_age,
            personal_data: p.personal_data,
            sensitive_data: p.sensitive_data,
            automated_decisions: p.automated_decisions,
            user_content: p.user_content,
            transfers: true,
            operates_service: true,
        };
        let measures = detect_requirements(&jurisdictions, &facts);
        let ids: Vec<&str> = measures.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"eu-transfer-safeguards"));
        assert!(ids.contains(&"eu-child-authorization"));
        assert!(ids.contains(&"uk-childrens-code"));
        assert!(ids.contains(&"us-ca-minor-opt-in"));
        assert!(
            !ids.contains(&"us-coppa-parental-consent"),
            "US not declared"
        );
        let transfer = measures
            .iter()
            .find(|m| m.id == "eu-transfer-safeguards")
            .unwrap();
        assert!(transfer
            .certified_methods
            .iter()
            .any(|m| m.id == "eu-scc-2021-914"));
        assert!(transfer
            .certified_methods
            .iter()
            .any(|m| m.id == "eu-us-data-privacy-framework"));
        let uk_child = measures
            .iter()
            .find(|m| m.id == "uk-child-authorization")
            .unwrap();
        assert!(uk_child.certified_methods.is_empty());
        for m in &measures {
            assert!(!m.triggered_by.is_empty());
        }
    }

    #[test]
    fn requirement_detection_finds_nothing_without_declared_facts() {
        let jurisdictions: BTreeSet<String> = ["EU".to_string()].into_iter().collect();
        let facts = MeasureFacts {
            personal_data: Some(false),
            ..Default::default()
        };
        assert!(detect_requirements(&jurisdictions, &facts).is_empty());
    }
}
