// Atlas OS — Step 1: Capture (RFC 23 §2.1).
//
// Phase 1: normalisation only. The raw prompt is trimmed, runs of whitespace
// are collapsed, and a single trailing newline (if any) is dropped. Nothing
// is mutated destructively — the original `raw_prompt` is preserved verbatim
// in the verdict's `raw_prompt` field, and the heuristic detectors below use
// the normalised version to avoid having to re-normalise on every match.

/// Produce a whitespace-normalised copy of the raw prompt.
pub fn normalise(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_ws = false;
    for ch in raw.trim().chars() {
        if ch.is_whitespace() {
            if !prev_ws {
                out.push(' ');
                prev_ws = true;
            }
        } else {
            out.push(ch);
            prev_ws = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::normalise;

    #[test]
    fn trims_and_collapses_runs() {
        assert_eq!(normalise("  fix   the  bug\n"), "fix the bug");
        assert_eq!(normalise("\tgo\t"), "go");
        assert_eq!(normalise("   "), "");
        assert_eq!(normalise("hi"), "hi");
    }
}
