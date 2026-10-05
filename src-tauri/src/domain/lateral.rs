// Atlas OS — lateral tool registry (RFC 64 §5; generalises RFC 28 §I).
//
// Every domain tool is an *external process*: probe (is it installed?), open
// (shell out), guide (install recipe). No bundling, no crates (RFC 25 §11).

use std::process::Command;

use super::manifest::DomainPack;

#[derive(Clone, Debug, PartialEq)]
pub struct LateralTool {
    pub id: String,
    pub bin: String,
    pub guide: String,
}

impl LateralTool {
    pub fn installed(&self) -> bool {
        probe_bin(&self.bin)
    }
}

/// Find an executable on `PATH` (plus the Windows `.exe/.cmd/.bat` suffixes).
pub fn probe_bin(bin: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path) {
        if dir.join(bin).is_file() {
            return true;
        }
        #[cfg(windows)]
        {
            for ext in ["exe", "cmd", "bat"] {
                if dir.join(format!("{bin}.{ext}")).is_file() {
                    return true;
                }
            }
        }
    }
    false
}

/// The lateral tools declared by a pack (its `[tools.lateral] open` list).
pub fn tools_for(pack: &DomainPack) -> Vec<LateralTool> {
    pack.tools
        .lateral
        .open
        .iter()
        .map(|name| LateralTool {
            id: name.clone(),
            bin: name.clone(),
            guide: guide_for(name),
        })
        .collect()
}

/// Install recipe for a known tool, or a generic fail-safe hint.
pub fn guide_for(name: &str) -> String {
    let known = match name.to_ascii_lowercase().as_str() {
        "freecad" => Some("https://www.freecad.org/downloads.php (or your OS package manager)"),
        "openscad" => Some("https://openscad.org/downloads.html"),
        "zoo" => Some("https://zoo.dev (Zoo KCL CLI)"),
        "blender" => Some("https://www.blender.org/download/"),
        "godot" => Some("https://godotengine.org/download"),
        "unity" => Some("Unity Hub: https://unity.com/download"),
        "qgis" => Some("https://qgis.org/download/"),
        _ => None,
    };
    match known {
        Some(url) => format!("install `{name}`: {url} — then ensure it is on PATH"),
        None => format!(
            "install `{name}` and ensure it is on PATH (RFC 64 lateral tool; Atlas never bundles it)"
        ),
    }
}

/// Shell out to a lateral tool. Fails with the install guide when the binary is
/// missing — never a silent no-op (RFC 28 §I discipline).
pub fn open(tool: &LateralTool, args: &[String]) -> anyhow::Result<std::process::ExitStatus> {
    if !tool.installed() {
        anyhow::bail!("`{}` is not installed. {}", tool.bin, tool.guide);
    }
    let status = Command::new(&tool.bin)
        .args(args)
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch `{}`: {e}", tool.bin))?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::parse_manifest;

    #[test]
    fn tools_follow_the_pack_open_list() {
        let pack = parse_manifest(
            "[domain]\nid=\"cad\"\ntitle=\"CAD\"\n[tools.lateral]\nopen=[\"freecad\",\"openscad\"]\n",
        )
        .unwrap();
        let tools = tools_for(&pack);
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].id, "freecad");
        assert!(tools[0].guide.contains("freecad"));
    }

    #[test]
    fn probe_finds_a_path_binary_and_misses_a_bogus_one() {
        // `cargo` is necessarily on PATH while `cargo test` runs.
        assert!(probe_bin("cargo"));
        assert!(!probe_bin("definitely-not-a-real-binary-xyz"));
    }

    #[test]
    fn open_on_missing_tool_guides_instead_of_silent_noop() {
        let tool = LateralTool {
            id: "nope".into(),
            bin: "definitely-not-a-real-binary-xyz".into(),
            guide: guide_for("definitely-not-a-real-binary-xyz"),
        };
        let err = open(&tool, &[]).unwrap_err().to_string();
        assert!(err.contains("not installed"));
        assert!(err.contains("PATH"));
    }
}
