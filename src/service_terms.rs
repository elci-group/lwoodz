// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
//! Terms-of-use analysis for dependencies that call an online service or
//! inference API rather than only ship source under an SPDX license.
//!
//! A dependency scanned by [`crate::manifest`] reports the SPDX license the
//! *crate/package itself* is distributed under. For a client SDK that talks
//! to a hosted service (Groq, OpenAI, a payments API, ...), that license
//! says nothing about the separate contract governing the runtime traffic —
//! the provider's terms of use, acceptable-use policy or API terms. Those
//! terms can restrict training use of submitted data, redistribution of
//! outputs, commercial use, or retention, independent of the SDK's own
//! license. This module flags dependencies that match a maintained catalog
//! of known online-service/inference providers and surfaces what is known
//! about their usage terms, so that gap is reviewed rather than silently
//! absent from a licensing audit.
use crate::assessment::EvidenceOrigin;
use crate::manifest::ManifestReport;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const SCHEMA: &str = "lwoodz.service-terms-analysis/v1";
pub const REVIEWED: &str = "2026-09-18";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceCategory {
    OnlineService,
    InferenceProvider,
}

/// Where a term stands, as declared by the provider's published terms —
/// never a measurement of actual runtime behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TermStatus {
    Permitted,
    Prohibited,
    RequiresOptOut,
    RequiresOptIn,
    Restricted,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageTerms {
    /// Whether the provider may use submitted input/output to train models.
    pub training_use: TermStatus,
    pub output_redistribution: TermStatus,
    pub commercial_use: TermStatus,
    pub data_retention: String,
    pub rate_limited: bool,
    pub attribution_required: bool,
    pub terms_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub provider: String,
    pub category: ServiceCategory,
    /// Lowercase substrings matched against a scanned dependency's name.
    pub match_patterns: Vec<String>,
    pub terms: UsageTerms,
    pub reviewed_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDependencyFinding {
    pub dependency: String,
    pub ecosystem: String,
    pub origin: EvidenceOrigin,
    pub provider: String,
    pub category: ServiceCategory,
    pub terms: UsageTerms,
    pub reviewed_on: String,
    pub confidence: f64,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: String,
    pub rules_reviewed_on: String,
    pub limitations: Vec<String>,
    pub findings: Vec<ServiceDependencyFinding>,
}

fn entry(
    provider: &str,
    category: ServiceCategory,
    patterns: &[&str],
    terms: UsageTerms,
) -> CatalogEntry {
    CatalogEntry {
        provider: provider.into(),
        category,
        match_patterns: patterns.iter().map(|s| (*s).into()).collect(),
        terms,
        reviewed_on: REVIEWED.into(),
    }
}

fn terms(
    training_use: TermStatus,
    output_redistribution: TermStatus,
    commercial_use: TermStatus,
    data_retention: &str,
    rate_limited: bool,
    attribution_required: bool,
    terms_url: &str,
) -> UsageTerms {
    UsageTerms {
        training_use,
        output_redistribution,
        commercial_use,
        data_retention: data_retention.into(),
        rate_limited,
        attribution_required,
        terms_url: terms_url.into(),
    }
}

/// The maintained set of known online-service and inference providers.
/// Match patterns are deliberately narrow (crate/package name fragments) to
/// avoid false positives against unrelated dependencies; extend this list
/// rather than widening a pattern when a new provider needs coverage.
pub fn catalog() -> Vec<CatalogEntry> {
    use ServiceCategory::{InferenceProvider, OnlineService};
    use TermStatus::{Permitted, Prohibited, RequiresOptOut, Restricted, Unknown};
    vec![
        entry("Groq", InferenceProvider, &["groq"], terms(
            RequiresOptOut, Restricted, Permitted,
            "Retained per provider policy; enterprise agreements may set a shorter window",
            true, false, "https://groq.com/terms-of-use/")),
        entry("OpenAI", InferenceProvider, &["openai"], terms(
            Prohibited, Restricted, Permitted,
            "API inputs/outputs not used for training by default; retained per provider policy",
            true, false, "https://openai.com/policies/usage-policies")),
        entry("Anthropic", InferenceProvider, &["anthropic"], terms(
            Prohibited, Restricted, Permitted,
            "API inputs/outputs not used for training by default; retained per provider policy",
            true, false, "https://www.anthropic.com/legal/aup")),
        entry("AWS Bedrock", InferenceProvider, &["bedrock", "aws-sdk-bedrock"], terms(
            Prohibited, Permitted, Permitted,
            "Governed by the applicable AWS service terms and data-processing addendum",
            true, false, "https://aws.amazon.com/service-terms/")),
        entry("Google Gemini / Vertex AI", InferenceProvider, &["vertexai", "vertex-ai", "generative-ai", "gemini"], terms(
            RequiresOptOut, Restricted, Permitted,
            "Varies between the free tier and paid/enterprise Vertex AI terms",
            true, false, "https://ai.google.dev/gemini-api/terms")),
        entry("Cohere", InferenceProvider, &["cohere"], terms(
            Prohibited, Restricted, Permitted,
            "Retained per provider policy",
            true, false, "https://cohere.com/terms-of-use")),
        entry("Hugging Face Inference", InferenceProvider, &["huggingface", "hf-hub", "hf_hub"], terms(
            Unknown, Restricted, Permitted,
            "Depends on the hosted model's own license/card; inference-API traffic follows Hugging Face's terms",
            true, false, "https://huggingface.co/terms-of-service")),
        entry("Replicate", InferenceProvider, &["replicate"], terms(
            Unknown, Restricted, Permitted,
            "Depends on the hosted model's own license; platform traffic follows Replicate's terms",
            true, false, "https://replicate.com/terms")),
        entry("Together AI", InferenceProvider, &["togetherai", "together-ai", "together_ai"], terms(
            RequiresOptOut, Restricted, Permitted,
            "Retained per provider policy",
            true, false, "https://www.together.ai/terms-of-service")),
        entry("Stripe", OnlineService, &["stripe"], terms(
            Prohibited, Restricted, Permitted,
            "Transaction and account data retained per Stripe's data policy and applicable law",
            true, false, "https://stripe.com/legal/consumer")),
        entry("Twilio", OnlineService, &["twilio"], terms(
            Prohibited, Restricted, Permitted,
            "Message/call metadata retained per Twilio's privacy and data-retention terms",
            true, false, "https://www.twilio.com/en-us/legal/tos")),
    ]
}

fn find_provider(dependency_name: &str) -> Option<&'static CatalogEntry> {
    // Leaked once into a 'static slice so the catalog is built a single
    // time and matches can borrow from it across the analysis.
    use std::sync::OnceLock;
    static CATALOG: OnceLock<Vec<CatalogEntry>> = OnceLock::new();
    let catalog = CATALOG.get_or_init(catalog);
    let lower = dependency_name.to_lowercase();
    catalog
        .iter()
        .find(|c| c.match_patterns.iter().any(|p| lower.contains(p.as_str())))
}

pub fn analyze(manifest: &ManifestReport) -> Report {
    let mut findings = Vec::new();
    for dep in &manifest.dependencies {
        if let Some(catalog_entry) = find_provider(&dep.name) {
            findings.push(ServiceDependencyFinding {
                dependency: dep.name.clone(),
                ecosystem: dep.source.clone(),
                origin: EvidenceOrigin::Observed,
                provider: catalog_entry.provider.clone(),
                category: catalog_entry.category,
                terms: catalog_entry.terms.clone(),
                reviewed_on: catalog_entry.reviewed_on.clone(),
                confidence: 0.7,
                notes: vec![format!(
                    "Matched by dependency name against the {} catalog entry; verify the actual integration path (SDK version, endpoint, region) before relying on these terms.",
                    catalog_entry.provider
                )],
            });
        }
    }
    findings.sort_by(|a, b| a.dependency.cmp(&b.dependency));
    Report {
        schema_version: SCHEMA.into(),
        rules_reviewed_on: REVIEWED.into(),
        limitations: vec![
            "Declared-context triage, not legal advice or a compliance certificate. Matching is by dependency name against a maintained catalog, not a scan of actual network traffic or contract terms.".into(),
            "Only providers already in the catalog are detected. A dependency that talks to an online or inference service without a recognizable SDK name (a direct HTTP client, an internal wrapper) is not found here.".into(),
            "Terms summarized per provider are a dated reference, not the current contract. Recheck the provider's published terms, any signed enterprise agreement, and the specific plan/region in use before relying on them.".into(),
            "A provider's terms govern the runtime service call; they are independent of, and do not change, the SPDX license the client SDK crate/package is itself distributed under.".into(),
        ],
        findings,
    }
}

pub fn render(report: &Report) -> String {
    let mut out = format!(
        "Service/inference terms-of-use analysis ({}) — catalog reviewed {}\n",
        report.schema_version, report.rules_reviewed_on
    );
    for note in &report.limitations {
        out.push_str(&format!("Note: {note}\n"));
    }
    if report.findings.is_empty() {
        out.push_str("\nNo dependencies matched the known online-service/inference catalog.\n");
        return out;
    }
    for f in &report.findings {
        out.push_str(&format!(
            "\n{} ({}) -> {} [{:?}]\n  Training use: {:?}  Output redistribution: {:?}  Commercial use: {:?}\n  Data retention: {}\n  Rate limited: {}  Attribution required: {}\n  Terms: {}\n  Reviewed: {}\n",
            f.dependency, f.ecosystem, f.provider, f.category,
            f.terms.training_use, f.terms.output_redistribution, f.terms.commercial_use,
            f.terms.data_retention, f.terms.rate_limited, f.terms.attribution_required,
            f.terms.terms_url, f.reviewed_on
        ));
        for n in &f.notes {
            out.push_str(&format!("  Note: {n}\n"));
        }
    }
    out
}

pub fn run(repo_root: &Path, json: bool) -> anyhow::Result<()> {
    let manifest = crate::manifest::scan(repo_root);
    let report = analyze(&manifest);
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
    use crate::manifest::Dependency;

    fn dep(name: &str, source: &str) -> Dependency {
        Dependency {
            name: name.into(),
            version: Some("1.0.0".into()),
            license: Some("MIT".into()),
            source: source.into(),
        }
    }

    #[test]
    fn matches_known_inference_and_service_dependencies_by_name() {
        let manifest = ManifestReport {
            dependencies: vec![
                dep("async-openai", "cargo"),
                dep("groq", "cargo"),
                dep("serde", "cargo"),
                dep("stripe", "npm"),
            ],
            sources: vec!["cargo".into(), "npm".into()],
        };
        let report = analyze(&manifest);
        let providers: Vec<&str> = report
            .findings
            .iter()
            .map(|f| f.provider.as_str())
            .collect();
        assert!(providers.contains(&"OpenAI"));
        assert!(providers.contains(&"Groq"));
        assert!(providers.contains(&"Stripe"));
        assert_eq!(
            report.findings.len(),
            3,
            "serde must not match any provider"
        );
    }

    #[test]
    fn unmatched_manifest_yields_empty_findings_not_an_error() {
        let manifest = ManifestReport {
            dependencies: vec![dep("serde", "cargo"), dep("tokio", "cargo")],
            sources: vec!["cargo".into()],
        };
        let report = analyze(&manifest);
        assert!(report.findings.is_empty());
        assert!(!report.limitations.is_empty());
    }

    #[test]
    fn every_catalog_entry_has_a_distinct_provider_and_nonempty_terms_url() {
        let catalog = catalog();
        let mut providers: Vec<&str> = catalog.iter().map(|c| c.provider.as_str()).collect();
        providers.sort();
        providers.dedup();
        assert_eq!(
            providers.len(),
            catalog.len(),
            "duplicate provider name in catalog"
        );
        for c in &catalog {
            assert!(
                !c.terms.terms_url.is_empty(),
                "{} missing a terms URL",
                c.provider
            );
            assert!(
                !c.match_patterns.is_empty(),
                "{} has no match patterns",
                c.provider
            );
        }
    }

    #[test]
    fn inference_dependency_flags_training_use_status_explicitly() {
        let manifest = ManifestReport {
            dependencies: vec![dep("groq", "cargo")],
            sources: vec!["cargo".into()],
        };
        let report = analyze(&manifest);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(
            report.findings[0].category,
            ServiceCategory::InferenceProvider
        );
        assert_ne!(report.findings[0].terms.training_use, TermStatus::Permitted);
    }

    #[test]
    fn render_includes_provider_and_terms_url_for_each_finding() {
        let manifest = ManifestReport {
            dependencies: vec![dep("anthropic-sdk", "npm")],
            sources: vec!["npm".into()],
        };
        let text = render(&analyze(&manifest));
        assert!(text.contains("Anthropic"));
        assert!(text.contains("anthropic.com"));
    }
}
