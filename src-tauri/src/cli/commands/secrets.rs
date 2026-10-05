// Atlas OS — `atlas secrets` (RFC 25 §3.10, RFC 18).
//
// Provider API keys live in the OS credential store, never in files or logs.
// This is the operator surface; the orchestrator resolves the same store as a
// fallback to env vars (`orchestrator::client::resolve_api_key`). The account
// is the key slot (by convention the provider's `api_key_env` name).

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};

use crate::profiles::{resolve_root, ProfileId};
use crate::secrets;

#[derive(Args, Debug)]
pub struct SecretsCmd {
    #[command(subcommand)]
    pub action: SecretsAction,
}

#[derive(Subcommand, Debug)]
pub enum SecretsAction {
    /// Store (or replace) a secret in the OS keychain.
    ///
    /// The value is read from stdin (pipe it: it never echoes into your shell
    /// history). Use `--value` only for non-sensitive throwaway values.
    Set {
        /// Key slot name, e.g. `OPENAI_API_KEY` (the provider's env var name).
        account: String,
        /// Secret value. Visible in shell history — prefer piping on stdin.
        #[arg(long)]
        value: Option<String>,
    },
    /// Read a secret (masked by default).
    Get {
        account: String,
        /// Print the raw secret instead of a masked preview.
        #[arg(long, default_value_t = false)]
        show: bool,
    },
    /// List the accounts recorded in this profile (names only, never values).
    List,
    /// Delete a secret from the OS keychain.
    Delete { account: String },
    /// Import secrets from a simple text file of `NOMBRE = valor` lines
    /// (open it in Notepad, write your key after the `=`). Blank values are
    /// skipped so the file can be a reusable template.
    Import {
        /// Path to the plain-text file.
        file: String,
        /// Blank the imported values in the file afterwards, so the plaintext
        /// key does not stay on disk.
        #[arg(long, default_value_t = false)]
        consume: bool,
    },
}

pub async fn run(cmd: SecretsCmd, profile: &str) -> Result<()> {
    let root = resolve_root(&ProfileId::new(profile))?;
    match cmd.action {
        SecretsAction::Set { account, value } => set(&root, &account, value),
        SecretsAction::Get { account, show } => get(&account, show),
        SecretsAction::List => list(&root),
        SecretsAction::Delete { account } => delete(&root, &account),
        SecretsAction::Import { file, consume } => import(&root, &file, consume),
    }
}

fn set(root: &Path, account: &str, value: Option<String>) -> Result<()> {
    let secret = match value {
        Some(v) if !v.trim().is_empty() => v,
        _ => read_secret_from_stdin()?,
    };
    if secret.trim().is_empty() {
        bail!("no secret provided — pipe it on stdin (`... | atlas secrets set {account}`) or pass --value");
    }
    secrets::set(account, &secret).map_err(|e| anyhow::anyhow!(e))?;
    remember_account(root, account)?;
    println!(
        "secrets: stored `{account}` in the OS keychain (service `{}`)",
        secrets::SERVICE
    );
    println!("  resolved by the orchestrator when `{account}` is not set as an env var");
    Ok(())
}

fn get(account: &str, show: bool) -> Result<()> {
    match secrets::get(account).map_err(|e| anyhow::anyhow!(e))? {
        Some(secret) => {
            if show {
                println!("{secret}");
            } else {
                println!("{account}: {}", secrets::preview(&secret));
                println!("  (pass --show to print the raw value)");
            }
        }
        None => println!("secrets: no entry for `{account}` in the OS keychain"),
    }
    Ok(())
}

fn list(root: &Path) -> Result<()> {
    let names = load_index(root)?;
    if names.is_empty() {
        println!(
            "secrets: no accounts recorded for this profile.\n  Add one with `atlas secrets set <account>`."
        );
        return Ok(());
    }
    println!(
        "secrets: {} account(s) recorded (OS keychain service `{}`)",
        names.len(),
        secrets::SERVICE
    );
    for name in names {
        let present = matches!(secrets::get(&name), Ok(Some(_)));
        println!("  {name}  [{}]", if present { "stored" } else { "missing" });
    }
    Ok(())
}

fn delete(root: &Path, account: &str) -> Result<()> {
    let removed = secrets::delete(account).map_err(|e| anyhow::anyhow!(e))?;
    forget_account(root, account)?;
    println!(
        "secrets: {} `{account}`",
        if removed {
            "deleted"
        } else {
            "no entry to delete for"
        }
    );
    Ok(())
}

/// Import a `NOMBRE = valor` text file into the keychain. `--consume` blanks
/// the values in the file afterwards so the plaintext key does not linger.
fn import(root: &Path, file: &str, consume: bool) -> Result<()> {
    let path = PathBuf::from(file);
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let raw = raw.strip_prefix('\u{feff}').unwrap_or(&raw); // strip a UTF-8 BOM
    let mut lines: Vec<String> = raw.lines().map(str::to_string).collect();

    let mut stored = 0usize;
    let mut empty = 0usize;
    for line in lines.iter_mut() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        if key.is_empty() {
            continue;
        }
        if value.is_empty() {
            empty += 1;
            continue;
        }
        secrets::set(key, value).map_err(|e| anyhow::anyhow!(e))?;
        remember_account(root, key)?;
        stored += 1;
        println!("secrets: stored `{key}`");
        if consume {
            *line = format!("{key} =");
        }
    }

    if consume && stored > 0 {
        std::fs::write(&path, lines.join("\n"))
            .with_context(|| format!("blanking {}", path.display()))?;
    }

    println!("secrets: {stored} secret(s) guardado(s), {empty} campo(s) vacío(s)");
    if stored == 0 {
        println!(
            "  (nada importado — escribe tu clave después del `=` en el archivo y vuelve a ejecutar)"
        );
    } else if consume {
        println!("  (los valores se borraron del archivo; la clave ya está en el Keychain)");
    }
    Ok(())
}

/// Read one line from stdin, without echoing it ourselves.
fn read_secret_from_stdin() -> Result<String> {
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .context("reading secret from stdin")?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

// A names-only index of what this profile has stored, so `list` can enumerate
// without reading (or guessing) secrets. The keychain has no enumeration API.

fn index_path(root: &Path) -> PathBuf {
    root.join("secrets.index.json")
}

fn load_index(root: &Path) -> Result<Vec<String>> {
    let path = index_path(root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut names: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
    names.sort();
    names.dedup();
    Ok(names)
}

fn save_index(root: &Path, names: &[String]) -> Result<()> {
    let path = index_path(root);
    let body = serde_json::to_string_pretty(names)?;
    let mut f =
        std::fs::File::create(&path).with_context(|| format!("writing {}", path.display()))?;
    f.write_all(body.as_bytes())?;
    Ok(())
}

fn remember_account(root: &Path, account: &str) -> Result<()> {
    let mut names = load_index(root)?;
    let account = account.trim().to_string();
    if !names.iter().any(|n| n == &account) {
        names.push(account);
        names.sort();
        save_index(root, &names)?;
    }
    Ok(())
}

fn forget_account(root: &Path, account: &str) -> Result<()> {
    let mut names = load_index(root)?;
    let before = names.len();
    names.retain(|n| n != account.trim());
    if names.len() != before {
        save_index(root, &names)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser, Debug)]
    struct TestCli {
        #[command(subcommand)]
        action: SecretsAction,
    }

    #[test]
    fn parses_set_with_value_and_without() {
        let c = TestCli::parse_from(["t", "set", "OPENAI_API_KEY", "--value", "sk-x"]);
        match c.action {
            SecretsAction::Set { account, value } => {
                assert_eq!(account, "OPENAI_API_KEY");
                assert_eq!(value.as_deref(), Some("sk-x"));
            }
            other => panic!("expected Set, got {other:?}"),
        }
        let c = TestCli::parse_from(["t", "set", "K"]);
        match c.action {
            SecretsAction::Set { value, .. } => assert!(value.is_none()),
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn parses_get_show_and_delete_and_list() {
        let c = TestCli::parse_from(["t", "get", "K", "--show"]);
        match c.action {
            SecretsAction::Get { account, show } => {
                assert_eq!(account, "K");
                assert!(show);
            }
            other => panic!("expected Get, got {other:?}"),
        }
        assert!(matches!(
            TestCli::parse_from(["t", "list"]).action,
            SecretsAction::List
        ));
        assert!(matches!(
            TestCli::parse_from(["t", "delete", "K"]).action,
            SecretsAction::Delete { .. }
        ));
    }

    #[test]
    fn index_roundtrips_names_only() {
        let dir = tempfile::TempDir::new().unwrap();
        remember_account(dir.path(), "A").unwrap();
        remember_account(dir.path(), "A").unwrap(); // idempotent
        remember_account(dir.path(), "B").unwrap();
        assert_eq!(load_index(dir.path()).unwrap(), vec!["A", "B"]);
        forget_account(dir.path(), "A").unwrap();
        assert_eq!(load_index(dir.path()).unwrap(), vec!["B"]);
        let body = std::fs::read_to_string(index_path(dir.path())).unwrap();
        assert!(!body.contains("secret"), "index holds names only");
    }
}
