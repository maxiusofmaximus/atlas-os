use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AstSymbolKind {
    Function,
    Class,
    Struct,
    Enum,
    Trait,
    Module,
    Method,
}

impl AstSymbolKind {
    pub fn tag(self) -> &'static str {
        match self {
            AstSymbolKind::Function => "function",
            AstSymbolKind::Class => "class",
            AstSymbolKind::Struct => "struct",
            AstSymbolKind::Enum => "enum",
            AstSymbolKind::Trait => "trait",
            AstSymbolKind::Module => "module",
            AstSymbolKind::Method => "method",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "function" => Some(AstSymbolKind::Function),
            "class" => Some(AstSymbolKind::Class),
            "struct" => Some(AstSymbolKind::Struct),
            "enum" => Some(AstSymbolKind::Enum),
            "trait" => Some(AstSymbolKind::Trait),
            "module" => Some(AstSymbolKind::Module),
            "method" => Some(AstSymbolKind::Method),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstSymbol {
    pub kind: AstSymbolKind,
    pub name: String,
    pub file: String,
    pub line: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AstError {
    #[error("symbol name is empty")]
    EmptyName,
    #[error("symbol line must be > 0")]
    BadLine,
    #[error("unknown file extension for '{0}'")]
    UnknownExtension(String),
}

impl AstSymbol {
    pub fn validate(&self) -> Result<(), AstError> {
        if self.name.trim().is_empty() {
            return Err(AstError::EmptyName);
        }
        if self.line == 0 {
            return Err(AstError::BadLine);
        }
        if language_of(&self.file).is_none() {
            return Err(AstError::UnknownExtension(self.file.clone()));
        }
        Ok(())
    }
}

pub fn language_of(path: &str) -> Option<&'static str> {
    match path.rsplit('.').next()? {
        "rs" => Some("rust"),
        "svelte" => Some("svelte"),
        "ts" | "tsx" | "mts" | "cts" => Some("typescript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        _ => None,
    }
}

pub const AST_PRESENCE_BOOST: f64 = 0.1;

pub fn presence_boost(present: bool) -> f64 {
    if present {
        AST_PRESENCE_BOOST
    } else {
        0.0
    }
}

pub fn confidence_for_symbol(base: f64, present: bool) -> f64 {
    (base + presence_boost(present)).clamp(0.0, 1.0)
}

pub fn names_set(symbols: &[AstSymbol]) -> HashSet<String> {
    symbols.iter().map(|s| s.name.clone()).collect()
}

pub fn symbols_for_file<'a>(symbols: &'a [AstSymbol], file: &str) -> Vec<&'a AstSymbol> {
    let mut out: Vec<&AstSymbol> = symbols.iter().filter(|s| s.file == file).collect();
    out.sort_by(|a, b| a.line.cmp(&b.line).then(a.name.cmp(&b.name)));
    out
}

pub fn extract(source: &str, file: &str) -> Vec<AstSymbol> {
    let lang = match language_of(file) {
        Some(l) => l,
        None => return Vec::new(),
    };
    if lang == "rust" {
        #[cfg(feature = "codebase-graph")]
        {
            let ts = extract_rs_ts(source, file);
            if !ts.is_empty() {
                return ts;
            }
        }
    }
    extract_heuristic(source, file)
}

pub fn extract_heuristic(source: &str, file: &str) -> Vec<AstSymbol> {
    if language_of(file).is_none() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (i, raw_line) in source.lines().enumerate() {
        let line_no = i as u32 + 1;
        let mut line = raw_line.trim();
        if line.is_empty()
            || line.starts_with("//")
            || line.starts_with('*')
            || line.starts_with('#')
            || line.starts_with('<')
            || line.starts_with('>')
            || line.starts_with('{')
            || line.starts_with('}')
            || line.starts_with('@')
            || line.starts_with("import ")
            || line.starts_with("import\t")
            || line.starts_with("use ")
            || line.starts_with("use\t")
        {
            continue;
        }
        line = strip_prefixes(line);
        let (head, rest) = match line.split_once(|c: char| c.is_whitespace()) {
            Some((h, r)) => (h, r.trim_start()),
            None => continue,
        };
        let kind = match head {
            "fn" | "function" => AstSymbolKind::Function,
            "struct" => AstSymbolKind::Struct,
            "enum" => AstSymbolKind::Enum,
            "trait" => AstSymbolKind::Trait,
            "mod" => AstSymbolKind::Module,
            "class" => AstSymbolKind::Class,
            "interface" => AstSymbolKind::Trait,
            _ => continue,
        };
        if let Some(name) = take_ident(rest) {
            out.push(AstSymbol {
                kind,
                name,
                file: file.to_string(),
                line: line_no,
            });
        }
    }
    out
}

fn strip_prefixes(mut line: &str) -> &str {
    loop {
        let next = if line.starts_with("pub")
            && line[3..].starts_with(|c: char| c.is_whitespace() || c == '(')
        {
            let mut rest = line[3..].trim_start();
            if rest.starts_with('(') {
                if let Some(end) = rest.find(')') {
                    rest = rest[end + 1..].trim_start();
                } else {
                    return line;
                }
            }
            rest
        } else if let Some(rest) = line
            .strip_prefix("async")
            .filter(|r| r.starts_with(|c: char| c.is_whitespace() || c == '('))
        {
            rest.trim_start()
        } else if let Some(rest) = line
            .strip_prefix("export")
            .filter(|r| r.starts_with(char::is_whitespace))
        {
            rest.trim_start()
        } else if let Some(rest) = line
            .strip_prefix("default")
            .filter(|r| r.starts_with(char::is_whitespace))
        {
            rest.trim_start()
        } else if let Some(rest) = line
            .strip_prefix("declare")
            .filter(|r| r.starts_with(char::is_whitespace))
        {
            rest.trim_start()
        } else if let Some(rest) = line
            .strip_prefix("abstract")
            .filter(|r| r.starts_with(char::is_whitespace))
        {
            rest.trim_start()
        } else {
            return line;
        };
        if next == line {
            return line;
        }
        line = next;
    }
}

fn take_ident(s: &str) -> Option<String> {
    let mut chars = s.chars();
    let first = chars.next()?;
    if !(first.is_alphabetic() || first == '_') {
        return None;
    }
    let mut name = String::from(first);
    for c in chars {
        if c.is_alphanumeric() || c == '_' {
            name.push(c);
        } else {
            break;
        }
    }
    Some(name)
}

#[cfg(feature = "codebase-graph")]
fn in_impl_block(mut node: tree_sitter::Node) -> bool {
    for _ in 0..4 {
        match node.parent() {
            Some(p) if p.kind() == "impl_item" => return true,
            Some(p) => node = p,
            None => return false,
        }
    }
    false
}

#[cfg(feature = "codebase-graph")]
pub fn extract_rs_ts(source: &str, file: &str) -> Vec<AstSymbol> {
    use tree_sitter::Parser;

    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .is_err()
    {
        return Vec::new();
    }
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(tree.root_node());
    while let Some(node) = queue.pop_front() {
        let kind = match node.kind() {
            "function_item" => {
                if in_impl_block(node) {
                    AstSymbolKind::Method
                } else {
                    AstSymbolKind::Function
                }
            }
            "struct_item" => AstSymbolKind::Struct,
            "enum_item" => AstSymbolKind::Enum,
            "trait_item" => AstSymbolKind::Trait,
            "mod_item" => AstSymbolKind::Module,
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    queue.push_back(child);
                }
                continue;
            }
        };
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(text) = name_node.utf8_text(source.as_bytes()) {
                if !text.is_empty() {
                    out.push(AstSymbol {
                        kind,
                        name: text.to_string(),
                        file: file.to_string(),
                        line: node.start_position().row as u32 + 1,
                    });
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            queue.push_back(child);
        }
    }
    out.sort_by(|a, b| a.line.cmp(&b.line).then(a.name.cmp(&b.name)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUST_SAMPLE: &str = "\
use std::collections::HashMap;\n\
\n\
pub fn alpha() {}\n\
async fn beta<T>(x: T) {}\n\
pub struct Config {}\n\
enum Level { Low }\n\
pub trait Store {}\n\
pub mod inner {}\n\
// fn ghost() {}\n\
";

    const SVELTE_SAMPLE: &str = "\
<script lang=\"ts\">\n\
  import { onMount } from 'svelte';\n\
  export function click() {}\n\
  export class Counter {}\n\
</script>\n\
";

    #[test]
    fn kind_tag_round_trip() {
        for v in [
            AstSymbolKind::Function,
            AstSymbolKind::Class,
            AstSymbolKind::Struct,
            AstSymbolKind::Enum,
            AstSymbolKind::Trait,
            AstSymbolKind::Module,
            AstSymbolKind::Method,
        ] {
            assert_eq!(AstSymbolKind::parse(v.tag()), Some(v));
        }
        assert!(AstSymbolKind::parse("ghost").is_none());
    }

    #[test]
    fn language_of_picks_known_extensions() {
        assert_eq!(language_of("a.rs"), Some("rust"));
        assert_eq!(language_of("A.svelte"), Some("svelte"));
        assert_eq!(language_of("a.ts"), Some("typescript"));
        assert_eq!(language_of("a.js"), Some("javascript"));
        assert_eq!(language_of("a.txt"), None);
        assert_eq!(language_of("noext"), None);
    }

    #[test]
    fn heuristic_extracts_rust_definitions() {
        let syms = extract_heuristic(RUST_SAMPLE, "lib.rs");
        let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        for want in ["alpha", "beta", "Config", "Level", "Store", "inner"] {
            assert!(names.contains(&want), "missing {want} in {names:?}");
        }
        assert!(
            !names.iter().any(|n| n.contains("HashMap") || *n == "ghost"),
            "imports and comments must not become symbols: {names:?}"
        );
        let by_name = |n: &str| syms.iter().find(|s| s.name == n).expect("symbol present");
        assert_eq!(by_name("alpha").kind, AstSymbolKind::Function);
        assert_eq!(by_name("Config").kind, AstSymbolKind::Struct);
        assert_eq!(by_name("Level").kind, AstSymbolKind::Enum);
        assert_eq!(by_name("Store").kind, AstSymbolKind::Trait);
        assert_eq!(by_name("inner").kind, AstSymbolKind::Module);
        assert_eq!(by_name("alpha").line, 3);
    }

    #[test]
    fn heuristic_extracts_svelte_exported_symbols() {
        let syms = extract_heuristic(SVELTE_SAMPLE, "Page.svelte");
        let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"click"), "names: {names:?}");
        assert!(names.contains(&"Counter"), "names: {names:?}");
        assert!(
            !names.iter().any(|n| n.contains("onMount")),
            "imports must not become symbols: {names:?}"
        );
    }

    #[test]
    fn extract_unknown_extension_returns_empty() {
        assert!(extract("fn ghost() {}", "notes.txt").is_empty());
        assert!(extract_heuristic("anything", "notes.txt").is_empty());
    }

    #[test]
    fn validate_rejects_empty_name_and_zero_line() {
        let bad_name = AstSymbol {
            kind: AstSymbolKind::Function,
            name: "  ".into(),
            file: "a.rs".into(),
            line: 1,
        };
        assert_eq!(bad_name.validate(), Err(AstError::EmptyName));
        let bad_line = AstSymbol {
            kind: AstSymbolKind::Function,
            name: "f".into(),
            file: "a.rs".into(),
            line: 0,
        };
        assert_eq!(bad_line.validate(), Err(AstError::BadLine));
        let bad_ext = AstSymbol {
            kind: AstSymbolKind::Function,
            name: "f".into(),
            file: "a.txt".into(),
            line: 1,
        };
        assert_eq!(
            bad_ext.validate(),
            Err(AstError::UnknownExtension("a.txt".into()))
        );
        let ok = AstSymbol {
            kind: AstSymbolKind::Struct,
            name: "S".into(),
            file: "a.rs".into(),
            line: 7,
        };
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn presence_boost_and_confidence_are_deterministic_and_clamped() {
        assert_eq!(presence_boost(true), AST_PRESENCE_BOOST);
        assert_eq!(presence_boost(false), 0.0);
        let a = confidence_for_symbol(0.7, true);
        let b = confidence_for_symbol(0.7, true);
        assert!((a - b).abs() < 1e-12);
        assert!((a - 0.8).abs() < 1e-9);
        assert_eq!(confidence_for_symbol(0.7, false), 0.7);
        assert_eq!(confidence_for_symbol(0.99, true), 1.0);
        assert_eq!(confidence_for_symbol(0.0, false), 0.0);
    }

    #[test]
    fn names_set_and_symbols_for_file() {
        let syms = extract_heuristic(RUST_SAMPLE, "lib.rs");
        let set = names_set(&syms);
        assert!(set.contains("alpha"));
        let scoped = symbols_for_file(&syms, "lib.rs");
        assert_eq!(scoped.len(), syms.len());
        assert!(symbols_for_file(&syms, "other.rs").is_empty());
        let lines: Vec<u32> = scoped.iter().map(|s| s.line).collect();
        let mut sorted = lines.clone();
        sorted.sort();
        assert_eq!(lines, sorted);
    }

    #[cfg(feature = "codebase-graph")]
    #[test]
    fn ts_extract_finds_rust_items_with_method_in_impl() {
        let src = "pub fn alpha() {}\nstruct S {}\nimpl S {\n fn boom(&self) {}\n}\n";
        let syms = extract_rs_ts(src, "lib.rs");
        let by_name = |n: &str| syms.iter().find(|s| s.name == n).expect("present");
        assert_eq!(by_name("alpha").kind, AstSymbolKind::Function);
        assert_eq!(by_name("S").kind, AstSymbolKind::Struct);
        assert_eq!(by_name("boom").kind, AstSymbolKind::Method);
    }

    #[cfg(feature = "codebase-graph")]
    #[test]
    fn extract_prefers_ts_over_heuristic_for_rust() {
        let syms = extract(RUST_SAMPLE, "lib.rs");
        assert!(syms.iter().any(|s| s.name == "alpha"));
        assert!(syms.iter().all(|s| s.validate().is_ok()));
    }
}
