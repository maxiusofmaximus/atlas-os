// Atlas OS — findings.json schema + validator (RFC 35 §3, Phase 9 sub-fase 9.0, M32).
//
// Machine-readable security findings in the Cloudflare `findings.json`
// pattern: a flat `AuditReport { findings }` document validated
// deterministically (no model call, no IO). The bridge
// `AuditReport::from_evidence` converts the existing EvidenceGate
// `EvidenceItem`s (Phase 2.5) into findings so `AuditReport::from_evidence`
// is the natural handoff from `collect_diff_evidence`.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::validation::evidence::{EvidenceItem, EvidenceKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn tag(&self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Critical => "critical",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "info" => Some(Severity::Info),
            "low" => Some(Severity::Low),
            "medium" => Some(Severity::Medium),
            "high" => Some(Severity::High),
            "critical" => Some(Severity::Critical),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub file: String,
    pub line: u32,
    pub evidence: String,
    pub remediation: String,
}

impl SecurityFinding {
    pub fn new(
        id: impl Into<String>,
        severity: Severity,
        title: impl Into<String>,
        file: impl Into<String>,
        line: u32,
        evidence: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            severity,
            title: title.into(),
            file: file.into(),
            line,
            evidence: evidence.into(),
            remediation: remediation.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditReport {
    #[serde(default)]
    pub findings: Vec<SecurityFinding>,
}

impl AuditReport {
    pub fn new(findings: Vec<SecurityFinding>) -> Self {
        Self { findings }
    }

    pub fn from_json_str(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }

    pub fn to_json_string_pretty(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn from_evidence(items: &[EvidenceItem]) -> Self {
        let findings = items
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let kind_tag = match e.kind {
                    EvidenceKind::ValidationReport => "validation_report",
                    EvidenceKind::TestOutput => "test_output",
                    EvidenceKind::TypecheckOutput => "typecheck_output",
                    EvidenceKind::LintOutput => "lint_output",
                    EvidenceKind::E2eOutput => "e2e_output",
                    EvidenceKind::ManualNote => "manual_note",
                };
                SecurityFinding::new(
                    format!("EV-{:04}", i + 1),
                    Severity::Info,
                    format!("evidence: {kind_tag}"),
                    "validation/evidence",
                    1,
                    e.reference.clone(),
                    "no remediation required — evidence record",
                )
            })
            .collect();
        Self { findings }
    }
}

pub fn validate_report(report: &AuditReport) -> Result<(), Vec<String>> {
    let mut errors: Vec<String> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for (i, f) in report.findings.iter().enumerate() {
        let ctx = format!("findings[{i}] (id='{}')", f.id);
        if f.id.trim().is_empty() {
            errors.push(format!("{ctx}: id is empty"));
        } else if !seen.insert(f.id.as_str()) {
            errors.push(format!("{ctx}: duplicate id '{}'", f.id));
        }
        if f.title.trim().is_empty() {
            errors.push(format!("{ctx}: title is empty"));
        }
        if f.file.trim().is_empty() {
            errors.push(format!("{ctx}: file is empty"));
        }
        if f.line == 0 {
            errors.push(format!("{ctx}: line must be > 0"));
        }
        if f.evidence.trim().is_empty() {
            errors.push(format!("{ctx}: evidence is empty"));
        }
        if f.remediation.trim().is_empty() {
            errors.push(format!("{ctx}: remediation is empty"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str) -> SecurityFinding {
        SecurityFinding::new(
            id,
            Severity::High,
            "unsafe unwrap in hot path",
            "src-tauri/src/core/pipeline.rs",
            42,
            "clippy::missing_errors_doc at pipeline.rs:42",
            "replace unwrap with anyhow context",
        )
    }

    #[test]
    fn schema_round_trip() {
        let report = AuditReport::new(vec![sample("SEC-001"), sample("SEC-002")]);
        let json = report.to_json_string_pretty().expect("serialize");
        let back = AuditReport::from_json_str(&json).expect("deserialize");
        assert_eq!(report, back);
        assert!(validate_report(&back).is_ok());
    }

    #[test]
    fn empty_report_is_valid() {
        let report = AuditReport::default();
        assert!(validate_report(&report).is_ok());
        let json = report.to_json_string_pretty().expect("serialize");
        assert!(json.contains("findings"));
    }

    #[test]
    fn empty_evidence_fails() {
        let mut f = sample("SEC-001");
        f.evidence = "   ".into();
        let report = AuditReport::new(vec![f]);
        let err = validate_report(&report).expect_err("empty evidence must fail");
        assert!(err.iter().any(|e| e.contains("evidence is empty")));
    }

    #[test]
    fn duplicate_ids_fail() {
        let report = AuditReport::new(vec![sample("SEC-001"), sample("SEC-001")]);
        let err = validate_report(&report).expect_err("dup ids must fail");
        assert!(err.iter().any(|e| e.contains("duplicate id")));
    }

    #[test]
    fn zero_line_fails() {
        let mut f = sample("SEC-001");
        f.line = 0;
        let report = AuditReport::new(vec![f]);
        let err = validate_report(&report).expect_err("line 0 must fail");
        assert!(err.iter().any(|e| e.contains("line must be > 0")));
    }

    #[test]
    fn invalid_severity_rejected_at_parse() {
        let bad = r#"{"findings":[{"id":"SEC-001","severity":"catastrophic","title":"t","file":"f.rs","line":1,"evidence":"e","remediation":"r"}]}"#;
        let err = AuditReport::from_json_str(bad).expect_err("bad severity must not parse");
        assert!(err.contains("catastrophic"), "unexpected error: {err}");
    }

    #[test]
    fn severity_tags_round_trip() {
        for s in [
            Severity::Info,
            Severity::Low,
            Severity::Medium,
            Severity::High,
            Severity::Critical,
        ] {
            assert_eq!(Severity::parse(s.tag()), Some(s));
        }
        assert_eq!(Severity::parse("catastrophic"), None);
    }

    #[test]
    fn from_evidence_maps_items_to_info_findings() {
        let items = vec![
            EvidenceItem::new(EvidenceKind::ValidationReport, "report-123"),
            EvidenceItem::new(EvidenceKind::TestOutput, "report-123:unit_tests"),
        ];
        let report = AuditReport::from_evidence(&items);
        assert_eq!(report.findings.len(), 2);
        assert_eq!(report.findings[0].id, "EV-0001");
        assert_eq!(report.findings[1].id, "EV-0002");
        assert!(report.findings.iter().all(|f| f.severity == Severity::Info));
        assert!(report.findings.iter().all(|f| f.line > 0));
        assert!(validate_report(&report).is_ok());
        assert_eq!(report.findings[0].evidence, "report-123");
    }

    #[test]
    fn from_evidence_empty_is_valid() {
        let report = AuditReport::from_evidence(&[]);
        assert!(report.findings.is_empty());
        assert!(validate_report(&report).is_ok());
    }
}
