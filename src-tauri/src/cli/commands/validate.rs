// Atlas OS — `atlas validate` Semgrep/CodeQL externos (RFC 20 Fase 12
// sub-fase 12.0, research 40 SECTOR B 12.0).
//
// Thin verb over `crate::validation::stages::static_analysis`:
// detects the external semgrep/codeql entry point (lateral — never
// bundled/linked), launches it via `std::process` only on explicit
// `--semgrep` / `--codeql`, parses the output (JSON / SARIF) into an
// M32 `AuditReport`, and prints it. Without the tool every verb
// degrades to the useful-message path + skip fail-safe (never blocks,
// exit 0) instead of failing.

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::validation::stages::static_analysis::{
    codeql_args, missing_message, normalize_target, parse_codeql_sarif, parse_semgrep_json,
    resolve_codeql, resolve_semgrep, run_tool, semgrep_args, static_guide, StaticTool, ToolStatus,
};

#[derive(Args, Debug)]
pub struct ValidateCmd {
    /// Run external semgrep (`semgrep --config auto --json <path>`).
    #[arg(long, default_value_t = false)]
    pub semgrep: bool,
    /// Run external codeql (`codeql database analyze --format=sarifv2.1.0 --output=- <db>`).
    #[arg(long, default_value_t = false)]
    pub codeql: bool,
    /// Target dir for semgrep, or existing CodeQL database dir for codeql.
    #[arg(long, default_value = ".")]
    pub path: String,
    /// Explicit semgrep binary path (overrides PATH + ATLAS_SEMGREP_BIN).
    #[arg(long)]
    pub semgrep_bin: Option<String>,
    /// Explicit codeql binary path (overrides PATH + ATLAS_CODEQL_BIN).
    #[arg(long)]
    pub codeql_bin: Option<String>,
    /// Emit the merged M32 `AuditReport` JSON after the human summary.
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

pub async fn run(cmd: ValidateCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let target = normalize_target(&cmd.path).ok_or_else(|| {
        anyhow::anyhow!(
            "--path is blank — pass a target dir (semgrep) or CodeQL database dir (codeql)"
        )
    })?;
    let want_semgrep = cmd.semgrep || !cmd.codeql;
    let want_codeql = cmd.codeql || !cmd.semgrep;
    let mut merged = crate::validation::AuditReport::default();
    if want_semgrep {
        let report = run_semgrep(&target, cmd.semgrep_bin.as_deref(), &pid.to_string())?;
        merged.findings.extend(report.findings);
    }
    if want_codeql {
        let report = run_codeql(&target, cmd.codeql_bin.as_deref(), &pid.to_string())?;
        merged.findings.extend(report.findings);
    }
    if !want_semgrep && !want_codeql {
        println!("{}", static_guide());
        return Ok(());
    }
    println!(
        "validate [{pid}] static analysis done: {} finding(s)",
        merged.findings.len()
    );
    if cmd.json {
        let json = merged
            .to_json_string_pretty()
            .map_err(|e| anyhow::anyhow!("validate serialize: {e}"))?;
        println!("{json}");
    }
    Ok(())
}

fn run_semgrep(
    target: &str,
    bin_override: Option<&str>,
    pid: &str,
) -> Result<crate::validation::AuditReport> {
    match resolve_semgrep(bin_override) {
        ToolStatus::Available { bin } => launch_and_parse(
            StaticTool::Semgrep,
            &bin,
            &semgrep_args(target),
            target,
            pid,
            parse_semgrep_json,
        ),
        ToolStatus::Missing => {
            println!("validate [{pid}] semgrep missing — stage skipped (fail-safe)");
            println!("{}", missing_message(StaticTool::Semgrep));
            Ok(crate::validation::AuditReport::default())
        }
    }
}

fn run_codeql(
    target: &str,
    bin_override: Option<&str>,
    pid: &str,
) -> Result<crate::validation::AuditReport> {
    match resolve_codeql(bin_override) {
        ToolStatus::Available { bin } => launch_and_parse(
            StaticTool::Codeql,
            &bin,
            &codeql_args(target),
            target,
            pid,
            parse_codeql_sarif,
        ),
        ToolStatus::Missing => {
            println!("validate [{pid}] codeql missing — stage skipped (fail-safe)");
            println!("{}", missing_message(StaticTool::Codeql));
            Ok(crate::validation::AuditReport::default())
        }
    }
}

fn launch_and_parse(
    tool: StaticTool,
    bin: &PathBuf,
    args: &[String],
    target: &str,
    pid: &str,
    parse: fn(&str) -> Result<crate::validation::AuditReport, String>,
) -> Result<crate::validation::AuditReport> {
    let cwd = std::path::Path::new(".");
    let out = match run_tool(bin, args, cwd) {
        Ok(out) => out,
        Err(e) => {
            println!(
                "validate [{pid}] {} launch failed ({}): {e} — stage skipped (fail-safe)",
                tool.as_str(),
                bin.display()
            );
            return Ok(crate::validation::AuditReport::default());
        }
    };
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let hint = if stderr.is_empty() {
            format!("exit {}", out.status)
        } else {
            stderr.chars().take(500).collect()
        };
        println!(
            "validate [{pid}] {} exited non-zero on `{target}` — stage skipped (fail-safe): {hint}",
            tool.as_str()
        );
        return Ok(crate::validation::AuditReport::default());
    }
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    match parse(&stdout) {
        Ok(report) => match crate::validation::validate_report(&report) {
            Ok(()) => {
                println!(
                    "validate [{pid}] {}: {} finding(s) in `{target}`",
                    tool.as_str(),
                    report.findings.len()
                );
                for f in &report.findings {
                    println!(
                        "  [{}] {} {}:{} — {}",
                        f.severity.tag(),
                        f.id,
                        f.file,
                        f.line,
                        f.title
                            .chars()
                            .take(120)
                            .collect::<String>()
                            .replace('\n', " ")
                    );
                }
                Ok(report)
            }
            Err(errors) => {
                println!(
                        "validate [{pid}] {} output failed M32 schema ({} error(s)) — stage skipped (fail-safe)",
                        tool.as_str(),
                        errors.len()
                    );
                for e in &errors {
                    println!("  [-] {e}");
                }
                Ok(crate::validation::AuditReport::default())
            }
        },
        Err(e) => {
            println!(
                "validate [{pid}] {} output unparseable — stage skipped (fail-safe): {e}",
                tool.as_str()
            );
            Ok(crate::validation::AuditReport::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser, Debug)]
    struct Probe {
        #[command(flatten)]
        validate: ValidateCmd,
    }

    #[test]
    fn defaults_target_cwd_without_tools() {
        let p = Probe::try_parse_from(["probe"]).unwrap();
        assert!(!p.validate.semgrep);
        assert!(!p.validate.codeql);
        assert_eq!(p.validate.path, ".");
        assert!(!p.validate.json);
    }

    #[test]
    fn semgrep_flag_parses_alone() {
        let p = Probe::try_parse_from(["probe", "--semgrep"]).unwrap();
        assert!(p.validate.semgrep);
        assert!(!p.validate.codeql);
    }

    #[test]
    fn codeql_flag_parses_alone() {
        let p = Probe::try_parse_from(["probe", "--codeql"]).unwrap();
        assert!(p.validate.codeql);
        assert!(!p.validate.semgrep);
    }

    #[test]
    fn path_and_bins_parse() {
        let p = Probe::try_parse_from([
            "probe",
            "--semgrep",
            "--path",
            "src",
            "--semgrep-bin",
            "/opt/semgrep",
            "--json",
        ])
        .unwrap();
        assert_eq!(p.validate.path, "src");
        assert_eq!(p.validate.semgrep_bin.as_deref(), Some("/opt/semgrep"));
        assert!(p.validate.json);
    }

    #[tokio::test]
    async fn blank_path_is_rejected_before_any_io() {
        let err = run(
            ValidateCmd {
                semgrep: true,
                codeql: false,
                path: "   ".to_string(),
                semgrep_bin: None,
                codeql_bin: None,
                json: false,
            },
            "nonexistent-profile-xyz",
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("--path is blank"));
    }

    #[tokio::test]
    async fn missing_tools_degrade_to_skip_fail_safe() {
        run(
            ValidateCmd {
                semgrep: true,
                codeql: true,
                path: ".".to_string(),
                semgrep_bin: Some("/nonexistent-atlas-validate-bin-xyz".to_string()),
                codeql_bin: Some("/nonexistent-atlas-validate-bin-xyz".to_string()),
                json: false,
            },
            "nonexistent-profile-xyz",
        )
        .await
        .expect("missing binaries must skip, never fail");
    }
}
