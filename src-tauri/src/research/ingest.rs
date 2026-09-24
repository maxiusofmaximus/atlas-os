// Atlas OS — Document ingestion (RFC 10 §6, Phase 3 sub-fase 3.2).
//
// anydoc single-binary-safety audit (RFC 25 §11, plan 30 §B 3.2): the
// upstream `firecrawl/anydoc` converter is a Node.js/Python-facing
// toolchain, not a published minimal-dependency Rust crate. Bundling it
// (or any equivalent converter crate with native deps) would break the
// 30–45 MB single-binary budget and the zero-native-deps rule, so the
// audit outcome is the plan's fallback branch: a minimal dependency-free
// parser owned by Atlas OS (plain text passthrough, CSV → Markdown
// table, uncompressed PDF text-object extraction) plus an opt-in
// external `pandoc` tool for the ZIP/legacy formats
// (docx/pptx/xlsx/odt/epub/rtf/…). `pandoc` is never bundled — it is
// resolved from `ATLAS_PANDOC_BIN` or `PATH` exactly like `ctx7max` in
// the 3.1 docs gateway, and a missing binary is a typed error, never a
// silent skip.
//
// Backend resolution per file extension:
//   * `md`/`markdown`/`txt` → UTF-8 passthrough (Markdown unchanged).
//   * `csv` → Markdown table (first row is the header).
//   * `pdf` → minimal text-object extraction (`(...)` literals + `<...>`
//     hex strings). Compressed (`FlateDecode`) and image-only pages
//     yield no text — reported honestly as short/empty Markdown per
//     the RFC 10 §10 fail-safe, never fabricated.
//   * office/epub/rtf/legacy → `pandoc <file> -t markdown` opt-in.
//   * anything else → `UnsupportedFormat` (typed, with the extension).
//
// Files over `MAX_BYTES` are rejected before reading. Every result maps
// to one `research_sources` row with `kind = "document"` (M25 already
// carries that kind); the `atlas research ingest` CLI (gated behind the
// `doc-ingest` Cargo feature, default off) owns the journal write so
// this module stays journal-agnostic and unit-testable without SQLite.

use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};

/// Hard cap on ingested file size (10 MiB). Rejected before reading so
/// a pasted export dump cannot OOM the desktop process.
pub const MAX_BYTES: u64 = 10 * 1024 * 1024;
/// Env var pinning the `pandoc` binary (operator override for installs
/// outside `PATH`; the `PATH` lookup remains the default).
pub const PANDOC_BIN_ENV: &str = "ATLAS_PANDOC_BIN";
/// Binary name probed in `PATH` for the opt-in converter.
pub const PANDOC_BIN: &str = "pandoc";
/// `research_sources.kind` value the CLI writes for ingested files (M25).
pub const DOCUMENT_KIND: &str = "document";

/// Classified input format. The wire form is snake_case so the CLI JSON
/// stays stable for downstream pipes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocFormat {
    Markdown,
    PlainText,
    Csv,
    Pdf,
    PandocAssisted { ext: String },
}

impl DocFormat {
    pub fn as_str(&self) -> String {
        match self {
            DocFormat::Markdown => "markdown".into(),
            DocFormat::PlainText => "plaintext".into(),
            DocFormat::Csv => "csv".into(),
            DocFormat::Pdf => "pdf".into(),
            DocFormat::PandocAssisted { ext } => format!("pandoc:{ext}"),
        }
    }

    pub fn needs_external_tool(&self) -> bool {
        matches!(self, DocFormat::PandocAssisted { .. })
    }
}

/// One ingested file: Markdown ready for the Research Engine plus the
/// provenance the CLI persists into `research_sources`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IngestedDocument {
    pub source_path: String,
    pub format: DocFormat,
    pub bytes: u64,
    pub markdown: String,
    pub title: Option<String>,
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum IngestError {
    #[error("document ingest: path must not be empty")]
    EmptyPath,
    #[error("document ingest: not found: {path}")]
    NotFound { path: String },
    #[error("document ingest: is a directory: {path}")]
    IsDirectory { path: String },
    #[error("document ingest: {bytes} bytes exceeds the {max} byte cap")]
    TooLarge { bytes: u64, max: u64 },
    #[error("document ingest: unsupported format{ext}")]
    UnsupportedFormat { ext: String },
    #[error("document ingest: `{tool}` is not installed; set ATLAS_PANDOC_BIN or install pandoc for .{ext} files")]
    ExternalToolMissing { tool: String, ext: String },
    #[error("document ingest: pandoc failed for `{path}`: {detail}")]
    PandocFailed { path: String, detail: String },
    #[error("document ingest: `{path}` is not valid UTF-8")]
    NotUtf8 { path: String },
    #[error("document ingest: io error for `{path}`: {detail}")]
    Io { path: String, detail: String },
}

/// Extensions the gateway accepts, lowercase without the dot.
pub fn supported_extensions() -> &'static [&'static str] {
    &[
        "md", "markdown", "txt", "csv", "pdf", "docx", "pptx", "xlsx", "odt", "ods", "odp", "epub",
        "rtf", "doc", "xls", "ppt",
    ]
}

fn extension_of(path: &Path) -> String {
    path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Classify a path by extension. Unknown or missing extensions are hard
/// errors — the gateway reports "unsupported", never a guessed answer.
pub fn detect_format(path: &Path) -> Result<DocFormat, IngestError> {
    let ext = extension_of(path);
    match ext.as_str() {
        "md" | "markdown" => Ok(DocFormat::Markdown),
        "txt" => Ok(DocFormat::PlainText),
        "csv" => Ok(DocFormat::Csv),
        "pdf" => Ok(DocFormat::Pdf),
        "docx" | "pptx" | "xlsx" | "odt" | "ods" | "odp" | "epub" | "rtf" | "doc" | "xls"
        | "ppt" => Ok(DocFormat::PandocAssisted { ext }),
        _ => Err(IngestError::UnsupportedFormat {
            ext: if ext.is_empty() {
                String::new()
            } else {
                format!(" (.{ext})")
            },
        }),
    }
}

fn io_error(path: &Path, e: std::io::Error) -> IngestError {
    if e.kind() == std::io::ErrorKind::NotFound {
        return IngestError::NotFound {
            path: path.display().to_string(),
        };
    }
    IngestError::Io {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}

/// Ingest one file into Markdown. Synchronous file I/O — the CLI calls
/// this from its async dispatcher via a direct call (no network, no
/// reason to hold the tokio runtime).
pub fn ingest_file(path: &Path) -> Result<IngestedDocument, IngestError> {
    if path.as_os_str().is_empty() {
        return Err(IngestError::EmptyPath);
    }
    let meta = std::fs::metadata(path).map_err(|e| io_error(path, e))?;
    if meta.is_dir() {
        return Err(IngestError::IsDirectory {
            path: path.display().to_string(),
        });
    }
    let bytes = meta.len();
    if bytes > MAX_BYTES {
        return Err(IngestError::TooLarge {
            bytes,
            max: MAX_BYTES,
        });
    }
    let format = detect_format(path)?;
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    let markdown = match &format {
        DocFormat::Markdown | DocFormat::PlainText => read_text(path)?,
        DocFormat::Csv => csv_to_markdown(&read_text(path)?),
        DocFormat::Pdf => {
            let raw = std::fs::read(path).map_err(|e| io_error(path, e))?;
            extract_pdf_text(&raw)
        }
        DocFormat::PandocAssisted { .. } => run_pandoc(path)?,
    };
    Ok(IngestedDocument {
        source_path: path.display().to_string(),
        format,
        bytes,
        markdown,
        title,
    })
}

fn read_text(path: &Path) -> Result<String, IngestError> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(IngestError::NotFound {
            path: path.display().to_string(),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => Err(IngestError::NotUtf8 {
            path: path.display().to_string(),
        }),
        Err(e) => Err(io_error(path, e)),
    }
}

/// Minimal CSV → Markdown table. First row is the header; cells escape
/// pipes. Handles quoted fields (`"a,b"`, `""""` escapes); unbalanced
/// quotes degrade to a plain split rather than an error.
pub fn csv_to_markdown(text: &str) -> String {
    let rows: Vec<Vec<String>> = text
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.trim().is_empty())
        .map(parse_csv_line)
        .collect();
    if rows.is_empty() {
        return String::new();
    }
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    let cell = |r: &[String], i: usize| {
        r.get(i)
            .map(|s| s.trim().replace('|', "\\|"))
            .unwrap_or_default()
    };
    let mut out = String::new();
    out.push_str("| ");
    for i in 0..width {
        if i > 0 {
            out.push_str(" | ");
        }
        out.push_str(&cell(&rows[0], i));
    }
    out.push_str(" |\n| ");
    for i in 0..width {
        if i > 0 {
            out.push_str(" | ");
        }
        out.push_str("---");
    }
    out.push_str(" |\n");
    for row in rows.iter().skip(1) {
        out.push_str("| ");
        for i in 0..width {
            if i > 0 {
                out.push_str(" | ");
            }
            out.push_str(&cell(row, i));
        }
        out.push_str(" |\n");
    }
    out
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                cur.push(c);
            }
        } else if c == '"' && cur.is_empty() {
            in_quotes = true;
        } else if c == ',' {
            fields.push(cur.clone());
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    fields.push(cur);
    fields
}

/// Minimal PDF text extraction: collects `(…)` literal strings (with
/// nesting, backslash escapes and octal codes) and `<…>` hex strings in
/// stream order, one per line. Covers uncompressed text objects — the
/// shape a smoke-test PDF and many spec exports carry.
pub fn extract_pdf_text(bytes: &[u8]) -> String {
    let mut pieces = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => {
                let (text, next) = read_literal(bytes, i);
                if !text.trim().is_empty() {
                    pieces.push(text);
                }
                i = next;
            }
            b'<' if i + 1 < bytes.len() && bytes[i + 1] != b'<' => {
                let (text, next) = read_hex(bytes, i);
                if !text.trim().is_empty() {
                    pieces.push(text);
                }
                i = next;
            }
            _ => i += 1,
        }
    }
    pieces.join("\n")
}

fn read_literal(bytes: &[u8], start: usize) -> (String, usize) {
    let mut out = String::new();
    let mut depth = 0usize;
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'(' {
            depth += 1;
            if depth > 1 {
                out.push('(');
            }
            i += 1;
        } else if b == b')' {
            depth -= 1;
            if depth == 0 {
                i += 1;
                break;
            }
            out.push(')');
            i += 1;
        } else if b == b'\\' && i + 1 < bytes.len() {
            let n = bytes[i + 1];
            match n {
                b'n' => out.push('\n'),
                b'r' => out.push('\r'),
                b't' => out.push('\t'),
                b'b' => out.push('\x08'),
                b'f' => out.push('\x0C'),
                b'(' => out.push('('),
                b')' => out.push(')'),
                b'\\' => out.push('\\'),
                b'\n' => {}
                b'\r' => {
                    if i + 2 < bytes.len() && bytes[i + 2] == b'\n' {
                        i += 1;
                    }
                }
                d if d.is_ascii_digit() => {
                    let mut val: u32 = (d - b'0') as u32;
                    let mut len = 1;
                    while len < 3
                        && i + 1 + len < bytes.len()
                        && bytes[i + 1 + len].is_ascii_digit()
                    {
                        val = val * 8 + (bytes[i + 1 + len] - b'0') as u32;
                        len += 1;
                    }
                    out.push(char::from_u32(val & 0xFF).unwrap_or('\u{FFFD}'));
                    i += len;
                }
                other => out.push(other as char),
            }
            i += 2;
        } else {
            out.push(if b.is_ascii() { b as char } else { '\u{FFFD}' });
            i += 1;
        }
    }
    (out, i)
}

fn read_hex(bytes: &[u8], start: usize) -> (String, usize) {
    let mut digits: Vec<u8> = Vec::new();
    let mut i = start + 1;
    while i < bytes.len() && bytes[i] != b'>' {
        let b = bytes[i];
        if b.is_ascii_hexdigit() {
            digits.push(b);
        }
        i += 1;
    }
    if i < bytes.len() {
        i += 1;
    }
    if digits.len() % 2 == 1 {
        digits.push(b'0');
    }
    let mut out = String::with_capacity(digits.len() / 2);
    for pair in digits.chunks(2) {
        let hi = (pair[0] as char).to_digit(16).unwrap_or(0);
        let lo = (pair[1] as char).to_digit(16).unwrap_or(0);
        let v = hi * 16 + lo;
        out.push(char::from_u32(v).unwrap_or('\u{FFFD}'));
    }
    (out, i)
}

/// Opt-in converter for office/epub/rtf/legacy formats. Resolves the
/// binary from `ATLAS_PANDOC_BIN` first, then `PATH` — the same
/// precedence the 3.1 docs gateway uses for `ctx7max`.
pub fn find_pandoc() -> Option<PathBuf> {
    if let Some(p) = env::var_os(PANDOC_BIN_ENV) {
        let p = PathBuf::from(p);
        if !p.as_os_str().is_empty() {
            return Some(p);
        }
    }
    let path = env::var_os("PATH")?;
    let names: &[&str] = if cfg!(windows) {
        &[PANDOC_BIN, "pandoc.exe"]
    } else {
        &[PANDOC_BIN]
    };
    for dir in env::split_paths(&path) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn run_pandoc(path: &Path) -> Result<String, IngestError> {
    let ext = extension_of(path);
    let Some(bin) = find_pandoc() else {
        return Err(IngestError::ExternalToolMissing {
            tool: PANDOC_BIN.into(),
            ext,
        });
    };
    let out = std::process::Command::new(&bin)
        .arg(path)
        .arg("-t")
        .arg("markdown")
        .output()
        .map_err(|e| IngestError::PandocFailed {
            path: path.display().to_string(),
            detail: e.to_string(),
        })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let detail = stderr.trim();
        let detail = if detail.is_empty() {
            format!("exit {}", out.status)
        } else if detail.len() > 300 {
            detail[..300].to_string()
        } else {
            detail.to_string()
        };
        return Err(IngestError::PandocFailed {
            path: path.display().to_string(),
            detail,
        });
    }
    match String::from_utf8(out.stdout) {
        Ok(s) => Ok(s),
        Err(_) => Err(IngestError::NotUtf8 {
            path: path.display().to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_file(name: &str, bytes: &[u8]) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::TempDir::new().unwrap();
        let p = dir.path().join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(bytes).unwrap();
        (dir, p)
    }

    #[test]
    fn markdown_passthrough_roundtrip() {
        let (_d, p) = tmp_file("spec.md", b"# Title\n\nBody text.\n");
        let doc = ingest_file(&p).unwrap();
        assert_eq!(doc.format, DocFormat::Markdown);
        assert_eq!(doc.markdown, "# Title\n\nBody text.\n");
        assert_eq!(doc.title.as_deref(), Some("spec"));
        assert_eq!(doc.bytes, 20);
        let json = serde_json::to_string(&doc).unwrap();
        let back: IngestedDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, back);
    }

    #[test]
    fn plaintext_passthrough_roundtrip() {
        let (_d, p) = tmp_file("notes.txt", b"hello world");
        let doc = ingest_file(&p).unwrap();
        assert_eq!(doc.format, DocFormat::PlainText);
        assert_eq!(doc.markdown, "hello world");
    }

    #[test]
    fn csv_becomes_markdown_table() {
        let (_d, p) = tmp_file("dump.csv", b"name,score\nada,97\n\"grace, h\",100\n");
        let doc = ingest_file(&p).unwrap();
        assert_eq!(doc.format, DocFormat::Csv);
        assert!(
            doc.markdown.contains("| name | score |"),
            "{:?}",
            doc.markdown
        );
        assert!(doc.markdown.contains("| --- | --- |"), "{:?}", doc.markdown);
        assert!(doc.markdown.contains("| ada | 97 |"), "{:?}", doc.markdown);
        assert!(
            doc.markdown.contains("| grace, h | 100 |"),
            "{:?}",
            doc.markdown
        );
    }

    #[test]
    fn pdf_literal_strings_extracted() {
        let pdf = b"%PDF-1.4\n1 0 obj<</Type/Catalog>>endobj\nBT /F1 12 Tf (Hello Atlas) Tj ET\nBT (Second line) Tj ET\ntrailer<</Root 1 0 R>>\n";
        let (_d, p) = tmp_file("spec.pdf", pdf);
        let doc = ingest_file(&p).unwrap();
        assert_eq!(doc.format, DocFormat::Pdf);
        assert!(doc.markdown.contains("Hello Atlas"), "{:?}", doc.markdown);
        assert!(doc.markdown.contains("Second line"), "{:?}", doc.markdown);
    }

    #[test]
    fn pdf_hex_strings_extracted() {
        let text = extract_pdf_text(b"<< /Type /Catalog >> <48656C6C6F> stream");
        assert!(text.contains("Hello"), "{text:?}");
    }

    #[test]
    fn pdf_literal_escapes_decoded() {
        let text = extract_pdf_text(b"BT (a\\(b\\) c\\n d\\101) Tj ET");
        assert!(text.contains("a(b)"), "{text:?}");
        assert!(text.contains("dA"), "{text:?}");
    }

    #[test]
    fn unsupported_format_is_typed_error() {
        let (_d, p) = tmp_file("blob.xyz", b"data");
        let err = ingest_file(&p).unwrap_err();
        assert_eq!(
            err,
            IngestError::UnsupportedFormat {
                ext: " (.xyz)".into()
            }
        );
    }

    #[test]
    fn missing_extension_is_unsupported() {
        let (_d, p) = tmp_file("README", b"data");
        assert!(matches!(
            ingest_file(&p).unwrap_err(),
            IngestError::UnsupportedFormat { .. }
        ));
    }

    #[test]
    fn empty_path_rejected() {
        assert_eq!(
            ingest_file(Path::new("")).unwrap_err(),
            IngestError::EmptyPath
        );
    }

    #[test]
    fn missing_file_reports_not_found() {
        let err = ingest_file(Path::new("/nonexistent-atlas-doc-xyz/missing.md")).unwrap_err();
        assert!(matches!(err, IngestError::NotFound { .. }), "{err:?}");
    }

    #[test]
    fn directory_rejected() {
        let dir = tempfile::TempDir::new().unwrap();
        let err = ingest_file(dir.path()).unwrap_err();
        assert!(matches!(err, IngestError::IsDirectory { .. }), "{err:?}");
    }

    #[test]
    fn office_format_without_pandoc_reports_missing_tool() {
        let (_d, p) = tmp_file("notes.docx", b"PK fake zip");
        match ingest_file(&p) {
            Ok(_) => {}
            Err(IngestError::ExternalToolMissing { tool, ext }) => {
                assert_eq!(tool, "pandoc");
                assert_eq!(ext, "docx");
            }
            Err(other) => panic!("expected ExternalToolMissing or Ok via pandoc, got {other:?}"),
        }
    }

    #[test]
    fn detect_format_classifies_each_family() {
        assert_eq!(
            detect_format(Path::new("a.md")).unwrap(),
            DocFormat::Markdown
        );
        assert_eq!(
            detect_format(Path::new("a.TXT")).unwrap(),
            DocFormat::PlainText
        );
        assert_eq!(detect_format(Path::new("a.csv")).unwrap(), DocFormat::Csv);
        assert_eq!(detect_format(Path::new("a.pdf")).unwrap(), DocFormat::Pdf);
        assert!(detect_format(Path::new("a.docx"))
            .unwrap()
            .needs_external_tool());
        assert!(detect_format(Path::new("a.odt"))
            .unwrap()
            .needs_external_tool());
    }

    #[test]
    fn format_wire_shape_is_snake_case() {
        let json =
            serde_json::to_string(&DocFormat::PandocAssisted { ext: "docx".into() }).unwrap();
        assert!(json.contains("pandoc_assisted"), "{json}");
        let json = serde_json::to_string(&DocFormat::PlainText).unwrap();
        assert_eq!(json, "\"plain_text\"");
    }

    #[test]
    fn supported_extensions_covers_plan_families() {
        let exts = supported_extensions();
        for must in [
            "md", "txt", "csv", "pdf", "docx", "pptx", "xlsx", "odt", "epub", "rtf",
        ] {
            assert!(exts.contains(&must), "missing {must}");
        }
    }
}
