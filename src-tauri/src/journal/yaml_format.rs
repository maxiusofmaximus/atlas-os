// OpenCode OS — Journal: YAML literal block formatter (RFC 28 §D — Fase 0 item PT-003).
//
// Ported from `darrenburns/posting/src/posting/yaml.py:str_presenter`
// (Apache-2.0, Copyright Darren Burns). The Python `str_presenter` registers a
// `yaml.add_representer(str, ...)` that emits multiline strings as YAML literal
// block scalars with style `|` and per-line `.rstrip()` to keep diffs clean.
//
// Rust port: pure-string `literal_block` produces the same logical output — it
// is the equivalent of `dumper.represent_scalar("tag:yaml.org,2002:str", data, style="|")`.
// Per §D of RFC 28, this is the canonical formatter for any audit-log entry that
// has a multi-line body (diff hunks, prompt text, transcripts). When we wire it
// into the actual `serde_yaml` emitter in Phase 1.5a, the emitter will consult
// this function to decide between plain scalar and literal-block scalar.
//
// Faithful port notes:
//   * Python's `data.count("\n") > 0` becomes `data.contains('\n')` (same)
//   * Python's `data.splitlines()` becomes Rust's `.lines()`. Both consume
//     trailing newline and split on `\n` (Python's splitlines additionally
//     splits on lone `\r`, `\r\n`, and Unicode line boundaries — but the
//     OpenCode OS Journal strips lone `\r` at INSERT time per RFC 02 §3.4,
//     so the divergence is contractually unreachable. Using `.lines()` also
//     gives Rust-native CRLF handling that matches Python's `splitlines()`
//     for `\r\n` inputs.
//   * `.trim_end()` per line mirrors Python's `line.rstrip()` exactly
//     (Python's rstrip with no args strips whitespace at the right end,
//      Rust's `trim_end` with no args does the same).
//   * Python's `"|"` style does NOT append a trailing newline after the
//     literal block; subsequent YAML emitters add one chomp marker. For our
//     standalone string output we DO NOT add a trailing newline either.

pub fn literal_block(data: &str) -> String {
    if !data.contains('\n') {
        return data.to_string();
    }
    data.lines()
        .map(|line| line.trim_end())
        .collect::<Vec<&str>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::literal_block;

    fn port_python_str_presenter(data: &str) -> String {
        if data.matches('\n').count() > 0 {
            let joined: Vec<&str> = data.lines().map(|l| l.trim_end()).collect();
            joined.join("\n")
        } else {
            data.to_string()
        }
    }

    #[test]
    fn no_newlines_returns_input_verbatim() {
        let s = "GET https://example.com HTTP/1.1";
        assert_eq!(literal_block(s), s);
        assert_eq!(literal_block(s), port_python_str_presenter(s));
    }

    #[test]
    fn empty_string_short_circuits() {
        assert_eq!(literal_block(""), "");
        assert_eq!(literal_block(""), port_python_str_presenter(""));
    }

    #[test]
    fn single_line_with_no_trailing_whitespace_unchanged() {
        let s = "diff --git a/foo b/foo";
        assert_eq!(literal_block(s), s);
    }

    #[test]
    fn multiline_strips_trailing_whitespace_per_line() {
        let input = "line1   \nline2\t\nline3";
        let expected = "line1\nline2\nline3";
        assert_eq!(literal_block(input), expected);
        assert_eq!(literal_block(input), port_python_str_presenter(input));
    }

    #[test]
    fn multiline_preserves_internal_whitespace() {
        let input = "  indented\n    deeper\nback";
        let expected = "  indented\n    deeper\nback";
        assert_eq!(literal_block(input), expected);
        assert_eq!(literal_block(input), port_python_str_presenter(input));
    }

    #[test]
    fn preserves_consecutive_blank_lines() {
        let input = "a\n\n\nb";
        let expected = "a\n\n\nb";
        assert_eq!(literal_block(input), expected);
        assert_eq!(literal_block(input), port_python_str_presenter(input));
    }

    #[test]
    fn trailing_newline_not_added_by_literal_block() {
        let input = "a\nb";
        let out = literal_block(input);
        assert!(!out.ends_with('\n'), "literal_block must NOT add trailing \\n (Python `|` scalar emits block, not single-line)");
        assert_eq!(out, "a\nb");
    }

    #[test]
    fn lone_carriage_return_treated_as_normal_char_per_contract() {
        let input = "a\rb";
        assert!(!input.contains('\n'));
        assert_eq!(literal_block(input), "a\rb");
    }

    #[test]
    fn crlf_treated_as_two_splits_per_python_semantics() {
        let input = "a\r\nb\r\nc";
        let out = literal_block(input);
        let py_equiv: String = input
            .lines()
            .map(|l| l.trim_end())
            .collect::<Vec<&str>>()
            .join("\n");
        assert_eq!(out, py_equiv);
        assert_eq!(out, "a\nb\nc");
    }

    #[test]
    fn unicode_content_preserved() {
        let input = "héllo\nwörld\n日本語";
        assert_eq!(literal_block(input), input);
        assert_eq!(literal_block(input), port_python_str_presenter(input));
    }

    #[test]
    fn diff_hunk_canonical_usecase() {
        let hunk = "--- a/foo.rs\n+++ b/foo.rs\n@@ -1,3 +1,4 @@\n fn main() {\n-    println!(\"old\");\n+    println!(\"new\");\n }\n";
        let out = literal_block(hunk);
        assert!(out.starts_with("--- a/foo.rs\n+++ b/foo.rs\n@@"));
        assert!(!out.ends_with('\n'));
        assert_eq!(out.lines().count(), hunk.lines().count());
        assert_eq!(out, port_python_str_presenter(hunk));
    }

    #[test]
    fn fuzz_random_multiline_matches_python_port() {
        let mut rng_seed = 0u64;
        for _ in 0..200 {
            rng_seed = rng_seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let lines_count = (rng_seed % 5) as usize + 1;
            let mut s = String::new();
            for i in 0..lines_count {
                if i > 0 {
                    s.push('\n');
                }
                let line_len = (rng_seed >> 32) as usize % 8;
                for _ in 0..line_len {
                    rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let c = match (rng_seed >> 60) % 4 {
                        0 => 'a',
                        1 => ' ',
                        2 => '\t',
                        _ => 'z',
                    };
                    s.push(c);
                }
            }
            assert_eq!(
                literal_block(&s),
                port_python_str_presenter(&s),
                "input: {:?}",
                s
            );
        }
    }
}
