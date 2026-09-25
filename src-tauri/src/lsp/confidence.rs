use serde::{Deserialize, Serialize};

use crate::context::ast::{confidence_for_symbol, AstSymbol};
use crate::journal::AstSymbolRow;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SymbolConfidence {
    pub name: String,
    pub file: String,
    pub line: u32,
    pub confidence: f64,
    pub present: bool,
}

impl SymbolConfidence {
    pub fn new(symbol: &AstSymbol, base: f64, present: bool) -> Option<Self> {
        if !base.is_finite() {
            return None;
        }
        symbol.validate().ok()?;
        Some(Self {
            name: symbol.name.clone(),
            file: symbol.file.clone(),
            line: symbol.line,
            confidence: confidence_for_symbol(base, present),
            present,
        })
    }

    pub fn hover_markdown(&self) -> String {
        format!(
            "**{}** `{}`:{} — Confidence: {:.2}",
            self.name, self.file, self.line, self.confidence
        )
    }

    pub fn diagnostic_message(&self) -> String {
        format!(
            "'{}' ({}:{}) confidence {:.2}",
            self.name, self.file, self.line, self.confidence
        )
    }
}

pub fn hover_for_symbol(symbol: &AstSymbol, base: f64, present: bool) -> Option<String> {
    SymbolConfidence::new(symbol, base, present).map(|s| s.hover_markdown())
}

pub fn diagnostic_for_symbol(symbol: &AstSymbol, base: f64, present: bool) -> Option<String> {
    SymbolConfidence::new(symbol, base, present).map(|s| s.diagnostic_message())
}

pub fn who_owns<'a>(rows: &'a [AstSymbolRow], name: &str) -> Vec<&'a AstSymbolRow> {
    let mut out: Vec<&AstSymbolRow> = rows.iter().filter(|r| r.name == name).collect();
    out.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.name.cmp(&b.name))
    });
    out
}

pub fn affects_where<'a>(rows: &'a [AstSymbolRow], file: &str) -> Vec<&'a AstSymbolRow> {
    let mut out: Vec<&AstSymbolRow> = rows.iter().filter(|r| r.file == file).collect();
    out.sort_by(|a, b| a.line.cmp(&b.line).then(a.name.cmp(&b.name)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ast::AstSymbolKind;
    use crate::skills::{Engine, SkillManifest};

    fn sym(name: &str, file: &str, line: u32) -> AstSymbol {
        AstSymbol {
            kind: AstSymbolKind::Function,
            name: name.into(),
            file: file.into(),
            line,
        }
    }

    fn row(file: &str, name: &str, line: i64) -> AstSymbolRow {
        AstSymbolRow {
            id: format!("{file}:{name}:{line}"),
            file: file.into(),
            name: name.into(),
            kind: "function".into(),
            line,
            lang: "rust".into(),
            extracted_at: "2026-09-25T00:00:00Z".into(),
        }
    }

    #[test]
    fn hover_returns_deterministic_confidence() {
        let s = sym("alpha", "lib.rs", 3);
        let a = SymbolConfidence::new(&s, 0.7, true).expect("valid");
        let b = SymbolConfidence::new(&s, 0.7, true).expect("valid");
        assert!((a.confidence - b.confidence).abs() < 1e-12);
        assert!((a.confidence - 0.8).abs() < 1e-9);
        assert!(a.present);
        let md = a.hover_markdown();
        assert!(md.contains("alpha"));
        assert!(md.contains("lib.rs"));
        assert!(md.contains(":3"));
        assert!(md.contains("0.80"));
        assert_eq!(hover_for_symbol(&s, 0.7, true), Some(md));
    }

    #[test]
    fn hover_wires_skill_picker_relevance_as_base() {
        let skill = SkillManifest {
            id: "react-ui-expert".into(),
            version: "1.0.0".into(),
            description: "Generacion de componentes React accesibles".into(),
            engine: Engine::Coding,
            priority: 90,
            verified: true,
            ..Default::default()
        };
        let base = crate::skills::picker::relevance("componentes react accesibles", &skill);
        let s = sym("click", "Page.svelte", 2);
        let conf = SymbolConfidence::new(&s, base, true).expect("valid");
        assert!((conf.confidence - confidence_for_symbol(base, true)).abs() < 1e-12);
        assert!((0.0..=1.0).contains(&conf.confidence));
    }

    #[test]
    fn diagnostic_exposes_confidence() {
        let s = sym("beta", "lib.rs", 5);
        let d = diagnostic_for_symbol(&s, 0.5, false).expect("valid");
        assert!(d.contains("beta"));
        assert!(d.contains("0.50"));
        let boosted = diagnostic_for_symbol(&s, 0.5, true).expect("valid");
        assert!(boosted.contains("0.60"));
    }

    #[test]
    fn failure_path_without_data_returns_none() {
        let empty_name = sym("  ", "lib.rs", 1);
        assert!(SymbolConfidence::new(&empty_name, 0.7, true).is_none());
        let zero_line = sym("alpha", "lib.rs", 0);
        assert!(SymbolConfidence::new(&zero_line, 0.7, true).is_none());
        let unknown_ext = sym("alpha", "notes.txt", 1);
        assert!(SymbolConfidence::new(&unknown_ext, 0.7, true).is_none());
        let ok = sym("alpha", "lib.rs", 1);
        assert!(SymbolConfidence::new(&ok, f64::NAN, true).is_none());
        assert!(SymbolConfidence::new(&ok, f64::INFINITY, true).is_none());
        assert!(hover_for_symbol(&empty_name, 0.7, true).is_none());
        assert!(diagnostic_for_symbol(&zero_line, 0.7, true).is_none());
    }

    #[test]
    fn confidence_clamps_to_unit_range() {
        let s = sym("alpha", "lib.rs", 1);
        let hi = SymbolConfidence::new(&s, 0.99, true).expect("valid");
        assert_eq!(hi.confidence, 1.0);
        let lo = SymbolConfidence::new(&s, 0.0, false).expect("valid");
        assert_eq!(lo.confidence, 0.0);
    }

    #[test]
    fn who_owns_and_affects_where_query_rows() {
        let rows = vec![
            row("lib.rs", "alpha", 30),
            row("lib.rs", "alpha", 3),
            row("other.rs", "alpha", 1),
            row("lib.rs", "zeta", 4),
        ];
        let owned = who_owns(&rows, "alpha");
        assert_eq!(owned.len(), 3);
        assert_eq!(owned[0].file, "lib.rs");
        assert_eq!(owned[0].line, 3);
        assert!(who_owns(&rows, "ghost").is_empty());
        let scoped = affects_where(&rows, "lib.rs");
        assert_eq!(scoped.len(), 3);
        let lines: Vec<i64> = scoped.iter().map(|r| r.line).collect();
        assert_eq!(lines, vec![3, 4, 30]);
        assert!(affects_where(&rows, "missing.rs").is_empty());
        assert!(who_owns(&[], "alpha").is_empty());
        assert!(affects_where(&[], "lib.rs").is_empty());
    }
}
