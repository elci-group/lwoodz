// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
//! Versioned, machine-readable licensing and provenance observations.
//!
//! These types describe evidence. They deliberately contain no recommendation
//! about whether software should be retained, changed, or replaced.

use crate::license::spdx::{normalize_spdx, SpdxExpression};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const LICENSE_ASSESSMENT_SCHEMA_VERSION: &str = "lwoodz.license-assessment/v1";
pub const PROVENANCE_ASSESSMENT_SCHEMA_VERSION: &str = "lwoodz.provenance-assessment/v1";
pub const EVIDENCE_SET_SCHEMA_VERSION: &str = "lwoodz.evidence-set/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOrigin {
    Declared,
    Observed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    LicenceDeclaration,
    LicenceText,
    CopyrightNotice,
    SourceRelationship,
    ThirdPartyMaterial,
    SourceSimilarity,
    VendoredSource,
    GeneratedArtifact,
    Wrapper,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenceFamily {
    Permissive,
    WeakCopyleft,
    StrongCopyleft,
    NetworkCopyleft,
    Proprietary,
    Unknown,
    Conflicting,
    Unverified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatentProvision {
    ExpressGrant,
    NoExpressGrant,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub id: String,
    pub subject: String,
    pub kind: EvidenceKind,
    pub origin: EvidenceOrigin,
    pub value: String,
    pub source: String,
    pub locator: Option<String>,
    pub content_hash: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicenceFact {
    pub subject: String,
    pub expression: String,
    pub spdx_identifiers: Vec<String>,
    pub family: LicenceFamily,
    pub origin: EvidenceOrigin,
    pub attribution_required: bool,
    pub redistribution_requirements: Vec<String>,
    pub modification_requirements: Vec<String>,
    pub patent_provision: PatentProvision,
    pub evidence_ids: Vec<String>,
    pub confidence: f64,
}

impl LicenceFact {
    #[must_use]
    pub fn normalized(
        subject: impl Into<String>,
        expression: impl Into<String>,
        family: LicenceFamily,
        origin: EvidenceOrigin,
        confidence: f64,
    ) -> Self {
        let expression = normalize_spdx(&expression.into());
        let spdx_identifiers = SpdxExpression::parse(expression.clone()).identifiers();
        Self {
            subject: subject.into(),
            expression,
            spdx_identifiers,
            family,
            origin,
            attribution_required: false,
            redistribution_requirements: Vec::new(),
            modification_requirements: Vec::new(),
            patent_provision: PatentProvision::Unknown,
            evidence_ids: Vec::new(),
            confidence,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopyrightFact {
    pub subject: String,
    pub notice: String,
    pub holders: Vec<String>,
    pub years: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceRelationship {
    DerivesFrom,
    CopiedFrom,
    GeneratedFrom,
    VendoredFrom,
    Wraps,
    Duplicates,
    SimilarTo,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceLink {
    pub subject: String,
    pub source: String,
    pub relationship: ProvenanceRelationship,
    pub evidence_ids: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicensingConflict {
    pub code: String,
    pub subject: String,
    pub declared_expressions: Vec<String>,
    pub observed_expressions: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicenseAssessment {
    pub schema_version: String,
    pub target: String,
    pub licences: Vec<LicenceFact>,
    pub copyrights: Vec<CopyrightFact>,
    pub conflicts: Vec<LicensingConflict>,
    pub confidence: f64,
}

impl LicenseAssessment {
    #[must_use]
    pub fn new(target: impl Into<String>, licences: Vec<LicenceFact>, confidence: f64) -> Self {
        let target = target.into();
        let conflicts = detect_repository_component_conflicts(&target, &licences);
        Self {
            schema_version: LICENSE_ASSESSMENT_SCHEMA_VERSION.to_string(),
            target,
            licences,
            copyrights: Vec::new(),
            conflicts,
            confidence,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceAssessment {
    pub schema_version: String,
    pub target: String,
    pub links: Vec<ProvenanceLink>,
    pub uncertain_subjects: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceSet {
    pub schema_version: String,
    pub license_assessment: LicenseAssessment,
    pub provenance_assessment: ProvenanceAssessment,
    pub observations: Vec<Observation>,
}

impl EvidenceSet {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != EVIDENCE_SET_SCHEMA_VERSION {
            return Err(format!(
                "unsupported evidence schema '{}'",
                self.schema_version
            ));
        }
        if self.license_assessment.schema_version != LICENSE_ASSESSMENT_SCHEMA_VERSION {
            return Err("unsupported licence assessment schema".to_string());
        }
        if self.provenance_assessment.schema_version != PROVENANCE_ASSESSMENT_SCHEMA_VERSION {
            return Err("unsupported provenance assessment schema".to_string());
        }
        if self.license_assessment.target != self.provenance_assessment.target {
            return Err("licensing and provenance targets differ".to_string());
        }
        let confidences = self
            .license_assessment
            .licences
            .iter()
            .map(|fact| fact.confidence)
            .chain(
                self.license_assessment
                    .conflicts
                    .iter()
                    .map(|item| item.confidence),
            )
            .chain(
                self.provenance_assessment
                    .links
                    .iter()
                    .map(|link| link.confidence),
            )
            .chain(
                self.observations
                    .iter()
                    .map(|observation| observation.confidence),
            )
            .chain([
                self.license_assessment.confidence,
                self.provenance_assessment.confidence,
            ]);
        if confidences
            .into_iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            return Err("confidence must be finite and between zero and one".to_string());
        }
        Ok(())
    }
}

#[must_use]
pub fn detect_repository_component_conflicts(
    repository: &str,
    licences: &[LicenceFact],
) -> Vec<LicensingConflict> {
    let declared: BTreeSet<String> = licences
        .iter()
        .filter(|fact| fact.subject == repository && fact.origin == EvidenceOrigin::Declared)
        .map(|fact| fact.expression.clone())
        .collect();
    if declared.is_empty() {
        return Vec::new();
    }

    licences
        .iter()
        .filter(|fact| fact.subject != repository && fact.origin == EvidenceOrigin::Observed)
        .filter(|fact| !declared.contains(&fact.expression))
        .map(|fact| LicensingConflict {
            code: "LICENSING_PROVENANCE_CONFLICT".to_string(),
            subject: fact.subject.clone(),
            declared_expressions: declared.iter().cloned().collect(),
            observed_expressions: vec![fact.expression.clone()],
            evidence_ids: fact.evidence_ids.clone(),
            confidence: fact.confidence,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_repository_component_licensing_conflict() {
        let declared = LicenceFact::normalized(
            "dependency-x",
            "MIT",
            LicenceFamily::Permissive,
            EvidenceOrigin::Declared,
            1.0,
        );
        let observed = LicenceFact::normalized(
            "dependency-x/component-f",
            "GPL-3.0-only",
            LicenceFamily::StrongCopyleft,
            EvidenceOrigin::Observed,
            0.96,
        );
        let assessment = LicenseAssessment::new("dependency-x", vec![declared, observed], 0.96);
        assert_eq!(assessment.conflicts.len(), 1);
        assert_eq!(
            assessment.conflicts[0].code,
            "LICENSING_PROVENANCE_CONFLICT"
        );
    }

    #[test]
    fn serializes_a_versioned_evidence_contract() {
        let assessment = LicenseAssessment::new("x", Vec::new(), 0.8);
        let set = EvidenceSet {
            schema_version: EVIDENCE_SET_SCHEMA_VERSION.to_string(),
            license_assessment: assessment,
            provenance_assessment: ProvenanceAssessment {
                schema_version: PROVENANCE_ASSESSMENT_SCHEMA_VERSION.to_string(),
                target: "x".to_string(),
                links: Vec::new(),
                uncertain_subjects: Vec::new(),
                confidence: 0.7,
            },
            observations: Vec::new(),
        };
        assert!(set.validate().is_ok());
        let json = serde_json::to_value(set).expect("evidence should serialize");
        assert_eq!(json["schema_version"], EVIDENCE_SET_SCHEMA_VERSION);
    }
}
