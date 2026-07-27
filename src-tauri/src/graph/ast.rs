// OpenCode OS — AST extractor for EXTRACTED provenance (RFC 28 §C).
//
// Gated behind `codebase-graph`. Uses tree-sitter with two grammars
// (rust, svelte) to extract call/import edges from the project's own
// sources — the "meta-skill" in RFC 28: OpenCode OS maps itself into
// a mission graph so the Learning Engine can flag when a new skill
// duplicates a kernel route.
//
// The extractor is deliberately narrow for Phase 1.5c:
//   * function calls  -> EdgeKind::Calls          provenance EXTRACTED
//   * use/static      -> EdgeKind::Imports        provenance EXTRACTED
//   * <script> import -> EdgeKind::Imports        provenance EXTRACTED
// Other edge kinds (transitions_to, depends_on, references) are the
// Planner / supervisor / Research Engine's job — never emitted here.
//
// Graphify's `find_import_cycles` is ported via DFS cycle detection
// over the imports edge set; the supervisor uses it to catch tight
// module cycles introduced by a Coding Engine diff before validation
// runs.

#![cfg_attr(not(feature = "codebase-graph"), allow(unused))]

#[cfg(feature = "codebase-graph")]
use tree_sitter::{Node as TSNode, Parser};

use crate::graph::{EdgeKind, Node, NodeId, NodeKind, Provenance};
use std::collections::HashSet;

/// Detected import cycle in the extracted edges. Returns the node ids
/// along the cycle starting at the entry, e.g. `[a, b, c, a]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportCycle(pub Vec<String>);

/// Source-code identifier extracted from a tree-sitter AST. The
/// `file` field is the relative path the symbol was found in (used as
/// a fallback edge label).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedSymbol {
    pub name: String,
    pub file: String,
    pub line: u32,
}

/// Wrapper so callers can carry Rust and Svelte symbols in the same
/// vec without parsing error on the wrong grammar.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Language {
    Rust,
    Svelte,
}

impl Language {
    /// File extension match used to pick a grammar.
    pub fn from_extension(path: &str) -> Option<Self> {
        let ext = path.rsplit('.').next()?;
        match ext {
            "rs" => Some(Self::Rust),
            "svelte" => Some(Self::Svelte),
            _ => None,
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Svelte => "svelte",
        }
    }
}

/// Edge extracted from a single parse pass. `weight` is 1 for an
/// uni-file reference and bumps to 2 when the call crosses file
/// boundaries — used by `find_import_cycles` to prefer intra-file
/// cycles when reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedEdge {
    pub src: String,
    pub dst: String,
    pub kind: EdgeKind,
    pub weight: u8,
}

/// Raw extraction payload returned by `extract_*`. Tracked by the
/// Journal when persisting to M15 — `provenance` is later patched to
/// INFERRED for AST nodes the LLM downgrades based on heuristics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractionReport {
    pub symbols: Vec<ExtractedSymbol>,
    pub edges: Vec<ExtractedEdge>,
}

impl ExtractionReport {
    pub fn merge(&mut self, other: ExtractionReport) {
        self.symbols.extend(other.symbols);
        self.edges.extend(other.edges);
    }
}

/// Find import cycles using DFS. Returns each SCC (strongly connected
/// component) whose size > 1, plus self-loops where a node imports
/// itself. Used by the supervisor to catch tight cycles a Coding
/// Engine diff may introduce before validation runs.
///
/// Tarjan's SCC — the standard single-pass approach (O(V+E)). The DFS
/// entry order is stable so duplicate runs on the same graph return
/// cycles in the same order, which keeps the audit trail deterministic.
pub fn find_import_cycles(edges: &[ExtractedEdge]) -> Vec<ImportCycle> {
    use std::collections::{HashMap, HashSet};

    let import_edges: Vec<(&str, &str)> = edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Imports)
        .map(|e| (e.src.as_str(), e.dst.as_str()))
        .collect();
    if import_edges.is_empty() {
        return Vec::new();
    }

    let mut nodes: Vec<&str> = import_edges
        .iter()
        .flat_map(|(s, d)| [*s, *d])
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    nodes.sort();

    let index_of: HashMap<&str, usize> = nodes.iter().enumerate().map(|(i, n)| (*n, i)).collect();
    let n = nodes.len();

    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (s, d) in &import_edges {
        if let (Some(&si), Some(&di)) = (index_of.get(s), index_of.get(d)) {
            adj[si].push(di);
        }
    }

    let mut index: Vec<usize> = vec![usize::MAX; n];
    let mut low: Vec<usize> = vec![0; n];
    let mut on_stack: Vec<bool> = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut counter: usize = 0;
    let mut cycles: Vec<ImportCycle> = Vec::new();
    let mut state = SccState {
        adj: &adj,
        nodes: &nodes,
        index: &mut index,
        low: &mut low,
        on_stack: &mut on_stack,
        stack: &mut stack,
        counter: &mut counter,
        cycles: &mut cycles,
    };

    for v in 0..n {
        if state.index[v] == usize::MAX {
            state.strongconnect(v);
        }
    }

    cycles
}

/// Mutable state shared across Tarjan recursion. Encapsulating the 9
/// pointer args into a struct keeps clippy's `too_many_arguments` lint
/// quiet without producing a noisy API — the struct is private to this
/// function's scope.
struct SccState<'a> {
    adj: &'a [Vec<usize>],
    nodes: &'a [&'a str],
    index: &'a mut [usize],
    low: &'a mut [usize],
    on_stack: &'a mut [bool],
    stack: &'a mut Vec<usize>,
    counter: &'a mut usize,
    cycles: &'a mut Vec<ImportCycle>,
}

impl<'a> SccState<'a> {
    fn strongconnect(&mut self, v: usize) {
        self.index[v] = *self.counter;
        self.low[v] = *self.counter;
        *self.counter += 1;
        self.stack.push(v);
        self.on_stack[v] = true;

        let neighbors: Vec<usize> = self.adj[v].clone();
        for w in neighbors {
            if self.index[w] == usize::MAX {
                self.strongconnect(w);
                self.low[v] = self.low[v].min(self.low[w]);
            } else if self.on_stack[w] {
                self.low[v] = self.low[v].min(self.index[w]);
            }
        }

        if self.low[v] != self.index[v] {
            return;
        }

        let mut scc: Vec<usize> = Vec::new();
        loop {
            let w = self.stack.pop().expect("scc stack not empty");
            self.on_stack[w] = false;
            scc.push(w);
            if w == v {
                break;
            }
        }
        let is_self_loop = scc.len() == 1 && self.adj[v].contains(&v);
        if scc.len() < 2 && !is_self_loop {
            return;
        }
        let mut labels: Vec<String> = scc.iter().map(|&i| self.nodes[i].to_string()).collect();
        labels.push(labels[0].clone());
        self.cycles.push(ImportCycle(labels));
    }
}

/// Convert an `ExtractionReport` into graph nodes + edges attached to
/// a mission. The mission root is left intact; each unique symbol
/// becomes a `Node { kind: External }` node (the climbed-out meta-skill
/// treats source symbols as out-of-graph dependencies during v1; we
/// switch to `EngineState` vs `Skill` in Phase 2 once the Planner owns
/// this mapping). Edges are emitted verbatim with `EXTRACTED`
/// provenance so the Learning Engine can down-rank them later.
pub fn report_to_graph(
    report: &ExtractionReport,
    mission_id: &str,
) -> (Vec<Node>, Vec<crate::graph::Edge>) {
    use crate::graph::{Edge, EdgeId};

    let mut nodes: Vec<Node> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for sym in &report.symbols {
        if seen.insert(sym.name.clone()) {
            nodes.push(Node {
                id: NodeId(format!("sym:{mission_id}:{}", sym.name)),
                mission_id: mission_id.to_string(),
                kind: NodeKind::External,
                label: sym.name.clone(),
                provenance: Provenance::Extracted,
                attrs_json: serde_json::json!({
                    "file": sym.file,
                    "line": sym.line,
                })
                .to_string(),
            });
        }
    }
    let edges: Vec<Edge> = report
        .edges
        .iter()
        .enumerate()
        .map(|(i, e)| Edge {
            id: EdgeId(format!("ext:{mission_id}:{i}")),
            mission_id: mission_id.to_string(),
            src: NodeId(format!("sym:{mission_id}:{}", e.src)),
            dst: NodeId(format!("sym:{mission_id}:{}", e.dst)),
            kind: e.kind,
            precondition: None,
            guard: None,
            visit_count: 0,
        })
        .collect();
    (nodes, edges)
}

#[cfg(feature = "codebase-graph")]
fn new_parser(lang: Language) -> Option<Parser> {
    let mut parser = Parser::new();
    let lang_obj = match lang {
        Language::Rust => tree_sitter_rust::LANGUAGE,
        Language::Svelte => tree_sitter_svelte_next::LANGUAGE,
    };
    parser.set_language(&lang_obj.into()).ok()?;
    Some(parser)
}

#[cfg(feature = "codebase-graph")]
fn walk<F: FnMut(TSNode)>(root: TSNode, mut visit: F) {
    use std::collections::VecDeque;
    let mut queue: VecDeque<TSNode> = VecDeque::new();
    queue.push_back(root);
    while let Some(node) = queue.pop_front() {
        visit(node);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            queue.push_back(child);
        }
    }
}

#[cfg(feature = "codebase-graph")]
fn extract_symbols_for_lang(source: &str, file: &str, lang: Language) -> ExtractionReport {
    // Svelte's tree-sitter grammar (PRRPCHT/tree-sitter-svelte-next)
    // tags the inside of <script> as `raw_text` — it doesn't embed a
    // JS/TS grammar. We fall back to two-line scanning with a tiny
    // regex-only heuristic for the imports block; the Rust branch keeps
    // the AST walk because tree-sitter-rust is real.
    if lang == Language::Svelte {
        return extract_svelte_heuristic(source, file);
    }

    let mut parser = match new_parser(lang) {
        Some(p) => p,
        None => return ExtractionReport::default(),
    };
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return ExtractionReport::default(),
    };
    let root = tree.root_node();
    let mut symbols = Vec::new();
    let mut edges = Vec::new();
    walk(root, |n| {
        let nt = n.kind();
        if matches!(
            nt,
            "function_item" | "function_signature_item" | "method_declaration"
        ) {
            if let Some(name) = n.child_by_field_name("name") {
                let text = name.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                if !text.is_empty() {
                    symbols.push(ExtractedSymbol {
                        name: text.clone(),
                        file: file.to_string(),
                        line: n.start_position().row as u32 + 1,
                    });
                    edges.push(ExtractedEdge {
                        src: file.to_string(),
                        dst: text,
                        kind: EdgeKind::References,
                        weight: 1,
                    });
                }
            }
        }
        let is_import_node = matches!(nt, "use_declaration" | "use_clause" | "import_statement")
            || nt.starts_with("import");
        if is_import_node {
            let text = n.utf8_text(source.as_bytes()).unwrap_or("").to_string();
            let trimmed = text.trim().trim_end_matches(';');
            if !trimmed.is_empty() {
                symbols.push(ExtractedSymbol {
                    name: trimmed.to_string(),
                    file: file.to_string(),
                    line: n.start_position().row as u32 + 1,
                });
                edges.push(ExtractedEdge {
                    src: file.to_string(),
                    dst: trimmed.to_string(),
                    kind: EdgeKind::Imports,
                    weight: 1,
                });
            }
        }
    });
    ExtractionReport { symbols, edges }
}

#[cfg(feature = "codebase-graph")]
fn extract_svelte_heuristic(source: &str, file: &str) -> ExtractionReport {
    let mut symbols = Vec::new();
    let mut edges = Vec::new();
    for (i, raw_line) in source.lines().enumerate() {
        let line = raw_line.trim();
        let line_no = i as u32 + 1;
        if line.starts_with("import ") || line.starts_with("import\t") {
            let trimmed = line.trim_end_matches(';').trim();
            if !trimmed.is_empty() {
                symbols.push(ExtractedSymbol {
                    name: trimmed.to_string(),
                    file: file.to_string(),
                    line: line_no,
                });
                edges.push(ExtractedEdge {
                    src: file.to_string(),
                    dst: trimmed.to_string(),
                    kind: EdgeKind::Imports,
                    weight: 1,
                });
            }
            continue;
        }
        if let Some(rest) = line
            .strip_prefix("function ")
            .or_else(|| line.strip_prefix("function\t"))
        {
            let name_end = rest
                .find(|c: char| c == '(' || c == '<' || c.is_whitespace())
                .unwrap_or(rest.len());
            let name = &rest[..name_end];
            if !name.is_empty() && name.chars().next().unwrap().is_alphabetic() {
                symbols.push(ExtractedSymbol {
                    name: name.to_string(),
                    file: file.to_string(),
                    line: line_no,
                });
                edges.push(ExtractedEdge {
                    src: file.to_string(),
                    dst: name.to_string(),
                    kind: EdgeKind::References,
                    weight: 1,
                });
            }
        }
    }
    ExtractionReport { symbols, edges }
}

/// Convenience entry: parse the given source text and return an
/// `ExtractionReport`. The `file` argument is purely for labelling —
/// we don't read disk from here (caller does) so tests can feed
/// in-memory snippets.
#[cfg(feature = "codebase-graph")]
pub fn extract(source: &str, file: &str) -> ExtractionReport {
    match Language::from_extension(file) {
        Some(lang) => extract_symbols_for_lang(source, file, lang),
        None => ExtractionReport::default(),
    }
}

#[cfg(all(test, feature = "codebase-graph"))]
mod tests {
    use super::*;

    const RUST_SAMPLE: &str = r#"
use std::collections::HashMap;

pub fn alpha() {}
fn beta() {}
"#;

    const SVELTE_SAMPLE: &str = r#"
<script lang="ts">
  import { onMount } from 'svelte';
  function click() {}
</script>
"#;

    #[test]
    fn language_from_extension_picks_known_extensions() {
        assert_eq!(Language::from_extension("foo/bar.rs"), Some(Language::Rust));
        assert_eq!(
            Language::from_extension("Foo.svelte"),
            Some(Language::Svelte)
        );
        assert_eq!(Language::from_extension("x.ts"), None);
        assert_eq!(Language::from_extension("noext"), None);
    }

    #[test]
    fn find_import_cycles_finds_self_loop() {
        let edges = vec![ExtractedEdge {
            src: "a".into(),
            dst: "a".into(),
            kind: EdgeKind::Imports,
            weight: 1,
        }];
        let cycles = find_import_cycles(&edges);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].0, vec!["a", "a"]);
    }

    #[test]
    fn find_import_cycles_finds_two_node_cycle() {
        let edges = vec![
            ExtractedEdge {
                src: "a".into(),
                dst: "b".into(),
                kind: EdgeKind::Imports,
                weight: 1,
            },
            ExtractedEdge {
                src: "b".into(),
                dst: "a".into(),
                kind: EdgeKind::Imports,
                weight: 1,
            },
        ];
        let cycles = find_import_cycles(&edges);
        assert_eq!(cycles.len(), 1);
        assert!(cycles[0].0.iter().any(|s| s == "a"));
        assert!(cycles[0].0.iter().any(|s| s == "b"));
    }

    #[test]
    fn find_import_cycles_returns_empty_for_acyclic() {
        let edges = vec![ExtractedEdge {
            src: "a".into(),
            dst: "b".into(),
            kind: EdgeKind::Imports,
            weight: 1,
        }];
        assert!(find_import_cycles(&edges).is_empty());
    }

    #[test]
    fn find_import_cycles_returns_empty_with_no_edges() {
        assert!(find_import_cycles(&[]).is_empty());
    }

    #[test]
    fn find_import_cycles_ignores_non_import_edges() {
        let edges = vec![ExtractedEdge {
            src: "a".into(),
            dst: "b".into(),
            kind: EdgeKind::Calls,
            weight: 1,
        }];
        assert!(find_import_cycles(&edges).is_empty());
    }

    #[test]
    fn report_to_graph_emits_symbol_nodes_and_edges() {
        let mut report = ExtractionReport::default();
        report.symbols.push(ExtractedSymbol {
            name: "alpha".into(),
            file: "lib.rs".into(),
            line: 1,
        });
        report.symbols.push(ExtractedSymbol {
            name: "alpha".into(),
            file: "lib.rs".into(),
            line: 1,
        });
        report.edges.push(ExtractedEdge {
            src: "lib.rs".into(),
            dst: "alpha".into(),
            kind: EdgeKind::Calls,
            weight: 1,
        });
        let (nodes, edges) = report_to_graph(&report, "m-1");
        // de-dup of symbol alpha
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id.as_str(), "sym:m-1:alpha");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, EdgeKind::Calls);
    }

    #[test]
    fn extract_rust_finds_functions_and_use() {
        let report = extract(RUST_SAMPLE, "lib.rs");
        let names: Vec<&str> = report.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"alpha"), "names: {names:?}");
        assert!(names.contains(&"beta"));
        assert!(
            names
                .iter()
                .any(|n| n.contains("use std::collections::HashMap")),
            "use_clause should be captured, names: {names:?}"
        );
    }

    #[test]
    fn extract_svelte_finds_imports_inside_script_block() {
        let report = extract(SVELTE_SAMPLE, "Page.svelte");
        let has_svelte_import = report.symbols.iter().any(|s| s.name.contains("onMount"));
        assert!(
            has_svelte_import,
            "svelte import not detected, symbols: {:?}",
            report.symbols
        );
    }

    #[test]
    fn extract_unknown_extension_returns_empty_report() {
        let report = extract("anything", "foo.txt");
        assert!(report.symbols.is_empty());
        assert!(report.edges.is_empty());
    }
}

#[cfg(all(test, not(feature = "codebase-graph")))]
mod tests {
    use super::*;

    #[test]
    fn find_import_cycles_works_without_tree_sitter_feature() {
        let edges = vec![ExtractedEdge {
            src: "a".into(),
            dst: "a".into(),
            kind: EdgeKind::Imports,
            weight: 1,
        }];
        let cycles = find_import_cycles(&edges);
        assert_eq!(cycles.len(), 1);
    }

    #[test]
    fn language_from_extension_works_without_tree_sitter_feature() {
        assert_eq!(Language::from_extension("x.rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("x.svelte"), Some(Language::Svelte));
        assert_eq!(Language::from_extension("x.txt"), None);
    }

    #[test]
    fn report_to_graph_works_without_tree_sitter_feature() {
        let mut report = ExtractionReport::default();
        report.symbols.push(ExtractedSymbol {
            name: "alpha".into(),
            file: "lib.rs".into(),
            line: 1,
        });
        report.edges.push(ExtractedEdge {
            src: "lib.rs".into(),
            dst: "alpha".into(),
            kind: EdgeKind::Calls,
            weight: 1,
        });
        let (nodes, edges) = report_to_graph(&report, "m-1");
        assert_eq!(nodes.len(), 1);
        assert_eq!(edges.len(), 1);
    }
}
