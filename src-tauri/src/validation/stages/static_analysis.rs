// Atlas OS — Semgrep/CodeQL stages externos (RFC 20 Fase 12
// sub-fase 12.0, research 40 SECTOR B 12.0 + RFC 14 §2).
//
// Semgrep + CodeQL son binarios pesados: Atlas OS JAMÁS los bundling,
// linkea, embebe o vendorea (RFC 25 §11). Integración lateral-only:
// cuando el operador ya los instaló, `atlas validate --semgrep` /
// `--codeql` detecta el entry point externo en `PATH` (o
// `ATLAS_SEMGREP_BIN` / `ATLAS_CODEQL_BIN`) y lo lanza como child
// process vía `std::process::Command`. Sin la herramienta cada verbo
// degrada a mensaje útil + skip fail-safe (el stage nunca bloquea por
// herramienta ausente) y jamás fabrica findings.
//
// El parse del output (JSON de semgrep / SARIF de codeql) produce un
// `AuditReport` (M32, sub-fase 9.0) — los findings machine-readable ya
// existen y `validate_report` los verifica.
//
// Cero crates nuevas (RFC 25 §11): detección sobre
// `std::env::split_paths`, launch sobre `std::process::Command`,
// parse sobre `serde_json` (ya en deps).

use std::env;
use std::path::PathBuf;

use crate::validation::report::{AuditReport, SecurityFinding, Severity};

/// Binary name probed in `PATH` when no explicit override is set.
pub const SEMGREP_BIN: &str = "semgrep";
/// Binary name probed in `PATH` when no explicit override is set.
pub const CODEQL_BIN: &str = "codeql";
/// Env var pinning the semgrep entry point (operator override for
/// installs outside `PATH`; the `PATH` lookup remains the default).
pub const SEMGREP_BIN_ENV: &str = "ATLAS_SEMGREP_BIN";
/// Env var pinning the codeql entry point (operator override for
/// installs outside `PATH`; the `PATH` lookup remains the default).
pub const CODEQL_BIN_ENV: &str = "ATLAS_CODEQL_BIN";
/// Where to get semgrep when it is missing (never fetched by Atlas).
pub const SEMGREP_INSTALL_HINT: &str =
    "https://semgrep.dev/docs/getting-started/ (`pip install semgrep` or the standalone binary)";
/// Where to get codeql when it is missing (never fetched by Atlas).
pub const CODEQL_INSTALL_HINT: &str =
    "https://codeql.github.com/docs/codeql-cli/getting-started-with-the-codeql-cli/";
/// Lateral-integration guardrail (RFC 25 §11 — jamás bundling).
pub const STATIC_LATERAL_NOTE: &str = "Semgrep/CodeQL are external tools — Atlas OS never bundles, links or embeds them; lateral external-process launch only.";

/// Which external static-analysis tool a verb targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticTool {
    Semgrep,
    Codeql,
}

impl StaticTool {
    pub fn as_str(self) -> &'static str {
        match self {
            StaticTool::Semgrep => "semgrep",
            StaticTool::Codeql => "codeql",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "semgrep" => Some(StaticTool::Semgrep),
            "codeql" => Some(StaticTool::Codeql),
            _ => None,
        }
    }

    pub fn bin_name(self) -> &'static str {
        match self {
            StaticTool::Semgrep => SEMGREP_BIN,
            StaticTool::Codeql => CODEQL_BIN,
        }
    }

    pub fn bin_env(self) -> &'static str {
        match self {
            StaticTool::Semgrep => SEMGREP_BIN_ENV,
            StaticTool::Codeql => CODEQL_BIN_ENV,
        }
    }

    pub fn install_hint(self) -> &'static str {
        match self {
            StaticTool::Semgrep => SEMGREP_INSTALL_HINT,
            StaticTool::Codeql => CODEQL_INSTALL_HINT,
        }
    }
}

/// What detection found on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolStatus {
    Available { bin: PathBuf },
    Missing,
}

impl ToolStatus {
    pub fn available(&self) -> bool {
        matches!(self, ToolStatus::Available { .. })
    }
}

/// Probe `PATH` for one binary name (Windows also tries `.exe`).
pub fn find_tool_in_path(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let names: Vec<String> = if cfg!(windows) {
        vec![name.to_string(), format!("{name}.exe")]
    } else {
        vec![name.to_string()]
    };
    for dir in env::split_paths(&path) {
        for n in &names {
            let candidate = dir.join(n);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn find_semgrep_in_path() -> Option<PathBuf> {
    find_tool_in_path(SEMGREP_BIN)
}

pub fn find_codeql_in_path() -> Option<PathBuf> {
    find_tool_in_path(CODEQL_BIN)
}

fn resolve_generic(bin_name: &str, bin_env: &str, bin_override: Option<&str>) -> ToolStatus {
    if let Some(b) = bin_override.map(str::trim).filter(|s| !s.is_empty()) {
        return ToolStatus::Available {
            bin: PathBuf::from(b),
        };
    }
    if let Ok(b) = env::var(bin_env) {
        let b = b.trim().to_string();
        if !b.is_empty() {
            return ToolStatus::Available {
                bin: PathBuf::from(b),
            };
        }
    }
    match find_tool_in_path(bin_name) {
        Some(bin) => ToolStatus::Available { bin },
        None => ToolStatus::Missing,
    }
}

/// Resolve one tool entry point: explicit override wins, then
/// `$ATLAS_*_BIN`, then `PATH`. Returns `Missing` instead of erroring
/// so the CLI prints the useful-message path (research 40 §12.0 "si no
/// está en PATH → mensaje útil + skip fail-safe").
pub fn resolve_tool(tool: StaticTool, bin_override: Option<&str>) -> ToolStatus {
    resolve_generic(tool.bin_name(), tool.bin_env(), bin_override)
}

pub fn resolve_semgrep(bin_override: Option<&str>) -> ToolStatus {
    resolve_tool(StaticTool::Semgrep, bin_override)
}

pub fn resolve_codeql(bin_override: Option<&str>) -> ToolStatus {
    resolve_tool(StaticTool::Codeql, bin_override)
}

/// Args for the external semgrep process (caller owns `Command`).
/// Fixed shape: `semgrep --config auto --json <target>`.
pub fn semgrep_args(target: &str) -> Vec<String> {
    vec![
        "--config".to_string(),
        "auto".to_string(),
        "--json".to_string(),
        target.trim().to_string(),
    ]
}

/// Args for the external codeql process (caller owns `Command`).
/// Fixed shape: `codeql database analyze --format=sarifv2.1.0
/// --output=- <db>`. `<db>` is an existing CodeQL database dir — the
/// operator creates it first (`codeql database create --language=...`);
/// Atlas never creates it.
pub fn codeql_args(db_path: &str) -> Vec<String> {
    vec![
        "database".to_string(),
        "analyze".to_string(),
        "--format=sarifv2.1.0".to_string(),
        "--output=-".to_string(),
        db_path.trim().to_string(),
    ]
}

/// Spawn an external static-analysis tool. Returns the raw `Output`
/// (caller parses stdout). The lateral boundary lives here:
/// `Command::new` on an operator-owned path, no link/bundle/embed.
pub fn run_tool(
    bin: &PathBuf,
    args: &[String],
    cwd: &std::path::Path,
) -> std::io::Result<std::process::Output> {
    let mut cmd = std::process::Command::new(bin);
    cmd.args(args);
    cmd.current_dir(cwd);
    cmd.output()
}

/// Validate the target string before spawning (blank targets never
/// reach the external process).
pub fn normalize_target(raw: &str) -> Option<String> {
    let t = raw.trim().to_string();
    if t.is_empty() {
        return None;
    }
    Some(t)
}

/// Map a semgrep `extra.severity` value to M32 `Severity`
/// (`ERROR` → High, `WARNING` → Medium, else Low).
pub fn semgrep_severity(raw: &str) -> Severity {
    match raw.trim().to_ascii_uppercase().as_str() {
        "ERROR" | "CRITICAL" | "HIGH" => Severity::High,
        "WARNING" | "MEDIUM" | "WARN" => Severity::Medium,
        "INFO" | "LOW" => Severity::Low,
        _ => Severity::Low,
    }
}

/// Map a SARIF `level` value to M32 `Severity` (`error` → High,
/// `warning` → Medium, `note`/`none` → Low).
pub fn codeql_level(raw: &str) -> Severity {
    match raw.trim().to_ascii_lowercase().as_str() {
        "error" => Severity::High,
        "warning" => Severity::Medium,
        "note" | "none" => Severity::Low,
        _ => Severity::Low,
    }
}

fn line_or_one(v: Option<u64>) -> u32 {
    match v {
        Some(n) if n >= 1 && n <= u32::MAX as u64 => n as u32,
        _ => 1,
    }
}

fn non_empty_or(s: &str, fallback: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        fallback.to_string()
    } else {
        t.to_string()
    }
}

/// Parse `semgrep --json` stdout into an M32 `AuditReport`.
/// Unknown shapes yield `Err` (caller prints the useful message);
/// well-formed output always validates via `validate_report`.
pub fn parse_semgrep_json(raw: &str) -> Result<AuditReport, String> {
    let v: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("semgrep: invalid JSON output: {e}"))?;
    let results = v
        .get("results")
        .and_then(|r| r.as_array())
        .ok_or_else(|| "semgrep: expected `results[]` in JSON output".to_string())?;
    let mut findings: Vec<SecurityFinding> = Vec::with_capacity(results.len());
    for (i, r) in results.iter().enumerate() {
        let check_id = r
            .get("check_id")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .trim();
        let id = if check_id.is_empty() {
            format!("SEMGREP-{i:04}")
        } else {
            check_id.to_string()
        };
        let path = r.get("path").and_then(|p| p.as_str()).unwrap_or("unknown");
        let line = line_or_one(
            r.get("start")
                .and_then(|s| s.get("line"))
                .and_then(|l| l.as_u64()),
        );
        let extra = r.get("extra");
        let message = extra
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let severity_raw = extra
            .and_then(|e| e.get("severity"))
            .and_then(|s| s.as_str())
            .unwrap_or("INFO");
        let evidence = non_empty_or(message, &id);
        let title = non_empty_or(check_id, &format!("semgrep finding {i}"));
        findings.push(SecurityFinding::new(
            id.clone(),
            semgrep_severity(severity_raw),
            title,
            non_empty_or(path, "unknown"),
            line,
            evidence,
            format!("see {id} docs (semgrep registry)"),
        ));
    }
    let report = AuditReport::new(findings);
    crate::validation::validate_report(&report).map_err(|e| e.join("; "))?;
    Ok(report)
}

/// Parse CodeQL SARIF (`--format=sarifv2.1.0`) stdout into an M32
/// `AuditReport`. Unknown shapes yield `Err`; well-formed SARIF always
/// validates via `validate_report`.
pub fn parse_codeql_sarif(raw: &str) -> Result<AuditReport, String> {
    let v: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("codeql: invalid SARIF output: {e}"))?;
    let runs = v
        .get("runs")
        .and_then(|r| r.as_array())
        .ok_or_else(|| "codeql: expected `runs[]` in SARIF output".to_string())?;
    let mut findings: Vec<SecurityFinding> = Vec::new();
    for run in runs {
        let empty = Vec::new();
        let results = run
            .get("results")
            .and_then(|r| r.as_array())
            .unwrap_or(&empty);
        for r in results {
            let idx = findings.len();
            let rule_id = r
                .get("ruleId")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    r.get("rule")
                        .and_then(|rule| rule.get("id"))
                        .and_then(|v| v.as_str())
                })
                .unwrap_or("")
                .trim();
            let id = if rule_id.is_empty() {
                format!("CODEQL-{idx:04}")
            } else {
                rule_id.to_string()
            };
            let message = r
                .get("message")
                .and_then(|m| m.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .trim();
            let level = r.get("level").and_then(|l| l.as_str()).unwrap_or("warning");
            let loc = r
                .get("locations")
                .and_then(|l| l.as_array())
                .and_then(|a| a.first())
                .and_then(|l| l.get("physicalLocation"));
            let uri = loc
                .and_then(|p| p.get("artifactLocation"))
                .and_then(|a| a.get("uri"))
                .and_then(|u| u.as_str())
                .unwrap_or("unknown");
            let line = line_or_one(
                loc.and_then(|p| p.get("region"))
                    .and_then(|rg| rg.get("startLine"))
                    .and_then(|l| l.as_u64()),
            );
            let evidence = non_empty_or(message, &id);
            findings.push(SecurityFinding::new(
                id.clone(),
                codeql_level(level),
                non_empty_or(rule_id, &format!("codeql finding {idx}")),
                non_empty_or(uri, "unknown"),
                line,
                evidence,
                format!("see CodeQL rule {id} docs"),
            ));
        }
    }
    let report = AuditReport::new(findings);
    crate::validation::validate_report(&report).map_err(|e| e.join("; "))?;
    Ok(report)
}

/// Message printed when the tool is not installed.
pub fn missing_message(tool: StaticTool) -> String {
    format!(
        "{name} no detectado en PATH ni en ${env} — stage omitido (fail-safe: jamás bloquea por herramienta ausente).\n\
         Instala {name} ({hint}) y reintenta: `atlas validate --{flag}`.\n\
         {lateral}"
        ,
        name = tool.as_str(),
        env = tool.bin_env(),
        hint = tool.install_hint(),
        flag = tool.as_str(),
        lateral = STATIC_LATERAL_NOTE
    )
}

/// Full static-analysis model text (install + semgrep/codeql flows +
/// M32 bridge) for the CLI guide path.
pub fn static_guide() -> String {
    format!(
        "Static analysis — Semgrep/CodeQL externos (RFC 20 Fase 12.0):\n\n\
         - Qué es: `atlas validate --semgrep` corre `semgrep --config auto --json <path>`; `atlas validate --codeql` corre `codeql database analyze --format=sarifv2.1.0 --output=- <db>`.\n\
         - CodeQL necesita una DB existente (`codeql database create --language=<lang> <db> <src>`); Atlas nunca la crea.\n\
         - El output se parsea a `AuditReport` (M32): `atlas audit validate <findings.json>` verifica el mismo schema.\n\
         - Sin la herramienta: mensaje útil + skip fail-safe (el stage nunca bloquea por herramienta ausente).\n\
         - Overrides: ${sem_env} / ${ql_env} o `--semgrep-bin` / `--codeql-bin`.\n\
         - {lateral}"
        ,
        sem_env = SEMGREP_BIN_ENV,
        ql_env = CODEQL_BIN_ENV,
        lateral = STATIC_LATERAL_NOTE
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_clean_env<F: FnOnce()>(f: F) {
        let _g = ENV_LOCK.lock().unwrap();
        let sem_b = env::var_os(SEMGREP_BIN_ENV);
        let ql_b = env::var_os(CODEQL_BIN_ENV);
        let path_b = env::var_os("PATH");
        env::remove_var(SEMGREP_BIN_ENV);
        env::remove_var(CODEQL_BIN_ENV);
        env::set_var("PATH", std::env::temp_dir().join("atlas-os-no-static-xyz"));
        f();
        if let Some(v) = sem_b {
            env::set_var(SEMGREP_BIN_ENV, v);
        } else {
            env::remove_var(SEMGREP_BIN_ENV);
        }
        if let Some(v) = ql_b {
            env::set_var(CODEQL_BIN_ENV, v);
        } else {
            env::remove_var(CODEQL_BIN_ENV);
        }
        if let Some(v) = path_b {
            env::set_var("PATH", v);
        } else {
            env::remove_var("PATH");
        }
    }

    #[test]
    fn tool_parse_round_trips() {
        assert_eq!(StaticTool::parse("semgrep"), Some(StaticTool::Semgrep));
        assert_eq!(StaticTool::parse("SEMGREP"), Some(StaticTool::Semgrep));
        assert_eq!(StaticTool::parse("codeql"), Some(StaticTool::Codeql));
        assert_eq!(StaticTool::parse("CODEQL"), Some(StaticTool::Codeql));
        assert_eq!(StaticTool::parse("snyk"), None);
        assert_eq!(StaticTool::parse(""), None);
        assert_eq!(StaticTool::Semgrep.as_str(), "semgrep");
        assert_eq!(StaticTool::Codeql.as_str(), "codeql");
    }

    #[test]
    fn find_unknown_tool_yields_none() {
        with_clean_env(|| {
            assert!(find_tool_in_path("atlas-os-no-such-tool-xyz").is_none());
        });
    }

    #[test]
    fn explicit_override_wins_over_path() {
        let st = resolve_semgrep(Some("/opt/semgrep/semgrep"));
        assert_eq!(
            st,
            ToolStatus::Available {
                bin: PathBuf::from("/opt/semgrep/semgrep"),
            }
        );
        let st = resolve_codeql(Some("/opt/codeql/codeql"));
        assert_eq!(
            st,
            ToolStatus::Available {
                bin: PathBuf::from("/opt/codeql/codeql"),
            }
        );
    }

    #[test]
    fn blank_override_falls_through_to_missing() {
        with_clean_env(|| {
            assert_eq!(resolve_semgrep(Some("   ")), ToolStatus::Missing);
            assert_eq!(resolve_codeql(Some("   ")), ToolStatus::Missing);
        });
    }

    #[test]
    fn empty_env_resolves_to_missing() {
        with_clean_env(|| {
            assert_eq!(resolve_semgrep(None), ToolStatus::Missing);
            assert_eq!(resolve_codeql(None), ToolStatus::Missing);
        });
    }

    #[test]
    fn env_override_selects_available() {
        with_clean_env(|| {
            env::set_var(SEMGREP_BIN_ENV, "/usr/local/bin/semgrep");
            assert_eq!(
                resolve_semgrep(None),
                ToolStatus::Available {
                    bin: PathBuf::from("/usr/local/bin/semgrep"),
                }
            );
            env::set_var(CODEQL_BIN_ENV, "/usr/local/bin/codeql");
            assert_eq!(
                resolve_codeql(None),
                ToolStatus::Available {
                    bin: PathBuf::from("/usr/local/bin/codeql"),
                }
            );
        });
    }

    #[test]
    fn semgrep_args_shape_is_fixed() {
        assert_eq!(
            semgrep_args("src"),
            vec![
                "--config".to_string(),
                "auto".to_string(),
                "--json".to_string(),
                "src".to_string(),
            ]
        );
    }

    #[test]
    fn codeql_args_shape_is_fixed() {
        assert_eq!(
            codeql_args("db"),
            vec![
                "database".to_string(),
                "analyze".to_string(),
                "--format=sarifv2.1.0".to_string(),
                "--output=-".to_string(),
                "db".to_string(),
            ]
        );
    }

    #[test]
    fn normalize_target_rejects_blank() {
        assert_eq!(normalize_target("  src "), Some("src".to_string()));
        assert_eq!(normalize_target("   "), None);
        assert_eq!(normalize_target(""), None);
    }

    #[test]
    fn severity_maps_cover_known_values() {
        assert_eq!(semgrep_severity("ERROR"), Severity::High);
        assert_eq!(semgrep_severity("warning"), Severity::Medium);
        assert_eq!(semgrep_severity("INFO"), Severity::Low);
        assert_eq!(semgrep_severity("???"), Severity::Low);
        assert_eq!(codeql_level("error"), Severity::High);
        assert_eq!(codeql_level("warning"), Severity::Medium);
        assert_eq!(codeql_level("note"), Severity::Low);
        assert_eq!(codeql_level("none"), Severity::Low);
        assert_eq!(codeql_level("???"), Severity::Low);
    }

    #[test]
    fn parse_semgrep_fixture_yields_m32_findings() {
        let raw = r#"{
            "results": [
                {"check_id": "python.lang.security.audit.exec-detected",
                 "path": "src/app.py", "start": {"line": 12},
                 "extra": {"message": "avoid exec()", "severity": "ERROR"}},
                {"check_id": "js.lang.security.audit.xss",
                 "path": "src/ui.js", "start": {"line": 7},
                 "extra": {"message": "possible XSS", "severity": "WARNING"}}
            ],
            "errors": []
        }"#;
        let report = parse_semgrep_json(raw).expect("fixture parses");
        assert_eq!(report.findings.len(), 2);
        assert_eq!(
            report.findings[0].id,
            "python.lang.security.audit.exec-detected"
        );
        assert_eq!(report.findings[0].file, "src/app.py");
        assert_eq!(report.findings[0].line, 12);
        assert_eq!(report.findings[0].severity, Severity::High);
        assert_eq!(report.findings[1].severity, Severity::Medium);
        assert!(crate::validation::validate_report(&report).is_ok());
    }

    #[test]
    fn parse_semgrep_empty_results_is_valid() {
        let report = parse_semgrep_json(r#"{"results": [], "errors": []}"#).expect("empty parses");
        assert!(report.findings.is_empty());
        assert!(crate::validation::validate_report(&report).is_ok());
    }

    #[test]
    fn parse_semgrep_invalid_json_errs() {
        assert!(parse_semgrep_json("not json").is_err());
        assert!(parse_semgrep_json(r#"{"nope": 1}"#).is_err());
    }

    #[test]
    fn parse_codeql_fixture_yields_m32_findings() {
        let raw = r#"{
            "version": "2.1.0",
            "runs": [{"results": [
                {"ruleId": "js/sql-injection", "level": "error",
                 "message": {"text": "SQL injection sink"},
                 "locations": [{"physicalLocation": {
                     "artifactLocation": {"uri": "src/db.js"},
                     "region": {"startLine": 33}}}]},
                {"ruleId": "py/unused-variable", "level": "note",
                 "message": {"text": "unused"},
                 "locations": [{"physicalLocation": {
                     "artifactLocation": {"uri": "src/a.py"},
                     "region": {"startLine": 2}}}]}
            ]}]
        }"#;
        let report = parse_codeql_sarif(raw).expect("fixture parses");
        assert_eq!(report.findings.len(), 2);
        assert_eq!(report.findings[0].id, "js/sql-injection");
        assert_eq!(report.findings[0].file, "src/db.js");
        assert_eq!(report.findings[0].line, 33);
        assert_eq!(report.findings[0].severity, Severity::High);
        assert_eq!(report.findings[1].severity, Severity::Low);
        assert!(crate::validation::validate_report(&report).is_ok());
    }

    #[test]
    fn parse_codeql_empty_runs_is_valid() {
        let report =
            parse_codeql_sarif(r#"{"version": "2.1.0", "runs": []}"#).expect("empty parses");
        assert!(report.findings.is_empty());
        assert!(crate::validation::validate_report(&report).is_ok());
    }

    #[test]
    fn parse_codeql_invalid_sarif_errs() {
        assert!(parse_codeql_sarif("not json").is_err());
        assert!(parse_codeql_sarif(r#"{"version": "2.1.0"}"#).is_err());
    }

    #[test]
    fn missing_message_points_to_install_and_env() {
        let msg = missing_message(StaticTool::Semgrep);
        assert!(msg.contains(SEMGREP_BIN_ENV));
        assert!(msg.contains("atlas validate --semgrep"));
        assert!(msg.contains("fail-safe"));
        assert!(msg.contains("never bundles"));
        let msg = missing_message(StaticTool::Codeql);
        assert!(msg.contains(CODEQL_BIN_ENV));
        assert!(msg.contains("atlas validate --codeql"));
    }

    #[test]
    fn run_missing_binary_surfaces_io_error_without_panic() {
        let bin = PathBuf::from("/nonexistent-atlas-static-bin-xyz");
        let err = run_tool(
            &bin,
            &["--version".to_string()],
            std::env::temp_dir().as_path(),
        )
        .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
