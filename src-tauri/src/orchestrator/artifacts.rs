// Atlas OS — ArtifactVerifier (RFC 63 §4 element 6).
//
// The terminal agent must not declare `done` on vibes. An `ArtifactCheck` is a
// declarative predicate the agent (or the task) states up front — "the file
// exists", "it contains this", "this command exits 0" — and `verify_artifacts`
// evaluates them all against the workspace. `done` is allowed only when every
// check passes, which is what turns the loop evidence-gated (RFC 14 §10 spirit,
// applied to agent artifacts rather than a `Diff`).
//
// Pure w.r.t. the model: it only reads the filesystem and runs commands through
// the injected `Sandbox`, so it is unit-testable offline.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::orchestrator::sandbox::Sandbox;

/// One declarative artifact predicate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ArtifactCheck {
    /// The path exists (file or dir).
    FileExists { path: String },
    /// The file exists and contains `needle`.
    FileContains { path: String, needle: String },
    /// A command exits 0 (run through the sandbox).
    CommandSucceeds {
        command: String,
        #[serde(default = "default_timeout_ms")]
        timeout_ms: u64,
    },
    /// The file's SHA-256 equals `sha256` (hex, lowercase).
    FileSha256 { path: String, sha256: String },
}

fn default_timeout_ms() -> u64 {
    120_000
}

/// Result of evaluating one check.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactResult {
    pub index: usize,
    pub passed: bool,
    pub detail: String,
}

/// Aggregate verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyVerdict {
    pub allowed: bool,
    pub results: Vec<ArtifactResult>,
    /// Human summary for the model / journal.
    pub summary: String,
}

impl VerifyVerdict {
    pub fn failures(&self) -> Vec<&ArtifactResult> {
        self.results.iter().filter(|r| !r.passed).collect()
    }
}

fn safe_join(root: &std::path::Path, rel: &str) -> Result<PathBuf, String> {
    if rel.trim().is_empty() {
        return Err("empty path".into());
    }
    let joined = root.join(rel);
    let mut out = PathBuf::new();
    for c in joined.components() {
        use std::path::Component::*;
        match c {
            CurDir => {}
            ParentDir => {
                if !out.pop() {
                    return Err("path escapes root".into());
                }
            }
            o => out.push(o.as_os_str()),
        }
    }
    if !out.starts_with(root) {
        return Err("path escapes root".into());
    }
    Ok(out)
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// Evaluate every check against `root`, running commands through `sandbox`.
pub fn verify_artifacts(
    root: &std::path::Path,
    checks: &[ArtifactCheck],
    sandbox: &dyn Sandbox,
) -> VerifyVerdict {
    let mut results = Vec::with_capacity(checks.len());
    for (index, check) in checks.iter().enumerate() {
        let r = match check {
            ArtifactCheck::FileExists { path } => match safe_join(root, path) {
                Ok(p) if p.exists() => ArtifactResult {
                    index,
                    passed: true,
                    detail: format!("{path} exists"),
                },
                Ok(_) => ArtifactResult {
                    index,
                    passed: false,
                    detail: format!("{path} does not exist"),
                },
                Err(e) => ArtifactResult {
                    index,
                    passed: false,
                    detail: e,
                },
            },
            ArtifactCheck::FileContains { path, needle } => match safe_join(root, path) {
                Ok(p) => match std::fs::read_to_string(&p) {
                    Ok(body) => ArtifactResult {
                        index,
                        passed: body.contains(needle),
                        detail: if body.contains(needle) {
                            format!("{path} contains the expected text")
                        } else {
                            format!("{path} does not contain the expected text")
                        },
                    },
                    Err(e) => ArtifactResult {
                        index,
                        passed: false,
                        detail: format!("read {path}: {e}"),
                    },
                },
                Err(e) => ArtifactResult {
                    index,
                    passed: false,
                    detail: e,
                },
            },
            ArtifactCheck::CommandSucceeds {
                command,
                timeout_ms,
            } => {
                let res = sandbox.exec(root, command, Duration::from_millis(*timeout_ms));
                ArtifactResult {
                    index,
                    passed: res.exit_code == 0,
                    detail: format!("`{command}` exit {}", res.exit_code),
                }
            }
            ArtifactCheck::FileSha256 { path, sha256 } => match safe_join(root, path) {
                Ok(p) => match std::fs::read(&p) {
                    Ok(bytes) => {
                        let got = sha256_hex(&bytes);
                        ArtifactResult {
                            index,
                            passed: got.eq_ignore_ascii_case(sha256),
                            detail: if got.eq_ignore_ascii_case(sha256) {
                                format!("{path} sha256 matches")
                            } else {
                                format!("{path} sha256 = {got} (expected {sha256})")
                            },
                        }
                    }
                    Err(e) => ArtifactResult {
                        index,
                        passed: false,
                        detail: format!("read {path}: {e}"),
                    },
                },
                Err(e) => ArtifactResult {
                    index,
                    passed: false,
                    detail: e,
                },
            },
        };
        results.push(r);
    }
    let allowed = results.iter().all(|r| r.passed);
    let failed = results.iter().filter(|r| !r.passed).count();
    let summary = if allowed {
        format!("all {} artifact check(s) passed", results.len())
    } else {
        format!("{failed}/{} artifact check(s) failed", results.len())
    };
    VerifyVerdict {
        allowed,
        results,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::sandbox::LocalSandbox;

    fn root_with(path: &str, content: &str) -> tempfile::TempDir {
        let d = tempfile::TempDir::new().unwrap();
        let p = d.path().join(path);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
        d
    }

    #[test]
    fn file_exists_and_contains_pass() {
        let d = root_with("out.txt", "hello world");
        let v = verify_artifacts(
            d.path(),
            &[
                ArtifactCheck::FileExists {
                    path: "out.txt".into(),
                },
                ArtifactCheck::FileContains {
                    path: "out.txt".into(),
                    needle: "world".into(),
                },
            ],
            &LocalSandbox,
        );
        assert!(v.allowed, "{v:?}");
    }

    #[test]
    fn relative_dot_is_rejected_as_a_root() {
        // Regression (RFC 63 §5): a `.` root makes `safe_join` treat the joined
        // path as escaping root — the host MUST canonicalize `--root` first.
        // This pins the contract so the CLI fix is not silently reverted.
        let v = verify_artifacts(
            std::path::Path::new("."),
            &[ArtifactCheck::FileExists {
                path: "out.txt".into(),
            }],
            &LocalSandbox,
        );
        assert!(!v.allowed, "a relative root must not silently pass");
    }

    #[test]
    fn missing_file_blocks() {
        let d = tempfile::TempDir::new().unwrap();
        let v = verify_artifacts(
            d.path(),
            &[ArtifactCheck::FileExists {
                path: "nope.txt".into(),
            }],
            &LocalSandbox,
        );
        assert!(!v.allowed);
        assert_eq!(v.failures().len(), 1);
    }

    #[test]
    fn wrong_content_blocks() {
        let d = root_with("out.txt", "abc");
        let v = verify_artifacts(
            d.path(),
            &[ArtifactCheck::FileContains {
                path: "out.txt".into(),
                needle: "zzz".into(),
            }],
            &LocalSandbox,
        );
        assert!(!v.allowed);
    }

    #[test]
    fn command_success_is_checked() {
        let d = tempfile::TempDir::new().unwrap();
        let ok = verify_artifacts(
            d.path(),
            &[ArtifactCheck::CommandSucceeds {
                command: "exit 0".into(),
                timeout_ms: 10_000,
            }],
            &LocalSandbox,
        );
        assert!(ok.allowed);
        let bad = verify_artifacts(
            d.path(),
            &[ArtifactCheck::CommandSucceeds {
                command: "exit 2".into(),
                timeout_ms: 10_000,
            }],
            &LocalSandbox,
        );
        assert!(!bad.allowed);
    }

    #[test]
    fn sha256_matches() {
        let d = root_with("f.txt", "abc");
        let expect = sha256_hex(b"abc");
        let v = verify_artifacts(
            d.path(),
            &[ArtifactCheck::FileSha256 {
                path: "f.txt".into(),
                sha256: expect,
            }],
            &LocalSandbox,
        );
        assert!(v.allowed);
    }

    #[test]
    fn path_escape_blocks() {
        let d = tempfile::TempDir::new().unwrap();
        let v = verify_artifacts(
            d.path(),
            &[ArtifactCheck::FileExists {
                path: "../x".into(),
            }],
            &LocalSandbox,
        );
        assert!(!v.allowed);
    }
}
