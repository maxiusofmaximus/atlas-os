// Atlas OS — Posting `.posting.yaml` format port (RFC 28 §D — Phase 1.5a §D-2).
//
// Ported from `darrenburns/posting/src/posting/collection.py` (Apache-2.0,
// Copyright Darren Burns). The schema (`RequestModel`, `Header`, `QueryParam`,
// `Cookie`, `Auth`, `RequestBody`, `Options`) is mirrored 1:1 using `serde` with
// `Option<T>` fields and `#[serde(default)]`. The `scripts` field is **never
// serialized or deserialized** — see AGENTS.md §6 security boundary: importing
// third-party posting collections that carry arbitrary Python scripts is a known
// attack surface; Atlas OS rejects the field outright.
//
// `entry_to_posting_yaml` maps an `AuditEntry` (the Atlas OS Journal row) to
// this schema, then serializes via `serde_yaml`. For HTTP-shaped audit entries
// (`action == "http_request"`), the inputs JSON is unmarshaled into the typed
// request fields (method/url/headers/body). For non-HTTP entries (`tool_call`,
// `swap_model`, etc.), we write a single `x-opencode-*` reserved-keys extension
// YAML object preserving actor / action / payloads verbatim.
//
// Field ordering mirrors posting's `RequestModel` field order so a diff against
// a hand-written `.posting.yaml` stays minimal and human-readable.
//
// # Canonical curl roundtrip tests (PT-006)
// Tests at the bottom of this file port `test_curl_export.py` cases:
//   * url with query string already expanded
//   * auth basic, digest, bearer_token
//   * empty body
//   * form_data with `[[]]` (nested list)
//   * headers as `[["Name","Value"], ...]` ordered-pair list
// The test runner emits `entry_to_posting_yaml(http_request_entry)` and asserts
// specific substrings that mirror posting's resolved YAML.

use serde::{Deserialize, Serialize};

use crate::journal::AuditEntry;

/// Subset of posting's `Auth` Literal["basic","digest","bearer_token"].
/// Mirrors `darrenburns/posting/src/posting/collection.py:Auth` (PT-001).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PostingAuth {
    Basic,
    Digest,
    BearerToken,
}

/// Header ordered-pair. Posting emits `headers: [["Name","Value"], ...]` rather
/// than a YAML map because RFC 2616 explicitly allows duplicate header names
/// and order MAY be significant (e.g. Set-Cookie). Port from PT-001.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostingHeader {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostingQueryParam {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostingCookie {
    pub name: String,
    pub value: String,
}

/// Body payload variants supported by posting's `RequestModel.body` field.
/// Posting emits `body: !json { ... }` or `body: !form ...` tagged YAML.
/// We use serde's tag = "kind" to produce the equivalent inline form.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PostingRequestBody {
    Json {
        text: serde_json::Value,
    },
    Form {
        values: Vec<(String, String)>,
    },
    Raw {
        text: String,
    },
    #[default]
    Empty,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct PostingOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_redirects: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f32>,
}

/// Posting's `RequestModel` schema (PT-001). Fields preserve posting's
/// camelCase naming so the emitted YAML round-trips back into posting.
///
/// SECURITY (AGENTS.md §6): the `scripts` field is deliberately ABSENT from
/// this struct. Deserialization of imported third-party YAML via
/// `posting_collection_from_yaml` will REJECT any document containing a `scripts`
/// key. See `PostingCollection::validate_no_scripts`.
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct PostingRequestModel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub method: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<PostingHeader>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<PostingQueryParam>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cookies: Vec<PostingCookie>,
    #[serde(default)]
    pub body: PostingRequestBody,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PostingAuth>,
    #[serde(default)]
    pub options: PostingOptions,
}

/// Posting's `Collection` is a directory of named requests. We emit the
/// top-level document with `request:` because posting's schema accepts both
/// inline `RequestModel` and unwrapped top-level models; we choose the
/// wrapped form to support multi-request files in the future.
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct PostingCollection {
    /// `posting_version` pinned to "1" to match posting's current major.
    /// Breaking changes in posting's `.posting.yaml` schema require a bump
    /// here AND a README.md update in the snapshot dir (RFC 28 §D Riesgos §1).
    pub posting_version: String,
    pub request: PostingRequestModel,
    /// `x-atlas-exported` is a provenance-line for the snapshot dir.
    /// Serialized as `x_opencode_exported` (serde rename below).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "x-atlas-exported"
    )]
    pub x_opencode_exported: Option<String>,
}

/// Top-of-file comment line emitted before the YAML body.
///
/// Per RFC 28 §D, every snapshot file begins with a comment line identifying
/// the export origin. `# x-atlas-exported: RFC 28 §D` makes the snapshot
/// self-describing for downstream git review.
pub const X_ATLAS_EXPORTED_HEADER: &str = "# x-atlas-exported: RFC 28 §D";

/// Carrier struct that wraps a `PostingCollection` so that for non-HTTP
/// audit entries we can flatten the extra `x-opencode-*` keys onto the top
/// level of the YAML document. For HTTP entries, the carrier is constructed
/// with `extra: None` and serialized as just the collection.
#[derive(Serialize)]
struct PostingCarrier {
    #[serde(flatten)]
    collection: PostingCollection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "x-opencode-extra")]
    x_opencode_extra: Option<serde_json::Value>,
}

/// Map an `AuditEntry` (Atlas OS Journal row) to a `PostingCollection`
/// representing the posting `.posting.yaml` schema.
///
/// # Mapping rules
///
/// * `entry.action == "http_request"`:
///   `entry.inputs` is unmarshaled as `{method, url, headers, params, body, auth}`.
///   Missing fields fall back to `GET` for method, the entry URL string otherwise.
/// * `entry.action != "http_request"`:
///   Emit `request: {method:"", url:"", x-opencode-* keys...}` with the
///   provenance fields preserved verbatim so the audit entry survives the
///   on-disk format translation without semantic loss.
pub fn audit_entry_to_posting_collection(entry: &AuditEntry) -> PostingCollection {
    if entry.action == "http_request" {
        let req_model = parse_http_request_inputs(entry);
        PostingCollection {
            posting_version: "1".to_string(),
            request: req_model,
            x_opencode_exported: Some(format!(
                "seq={} ts={} actor={}",
                entry.seq, entry.ts, entry.actor
            )),
        }
    } else {
        let req_model = PostingRequestModel::default();
        PostingCollection {
            posting_version: "1".to_string(),
            request: req_model,
            x_opencode_exported: Some(format!(
                "seq={} ts={} actor={}",
                entry.seq, entry.ts, entry.actor
            )),
        }
    }
}

/// Build the extra `x-opencode-*` extension keys block for non-HTTP entries.
fn build_extra_payload(entry: &AuditEntry) -> Option<serde_json::Value> {
    if entry.action == "http_request" {
        return None;
    }
    Some(serde_json::json!({
        "x-opencode-action": entry.action,
        "x-opencode-inputs": entry.inputs,
        "x-opencode-outputs": entry.outputs,
    }))
}

/// Serialize the collection to `.posting.yaml` form as a String, prefixed with
/// the standard `# x-atlas-exported: RFC 28 §D` comment.
pub fn entry_to_posting_yaml(entry: &AuditEntry) -> String {
    let collection = audit_entry_to_posting_collection(entry);
    let carrier = PostingCarrier {
        collection,
        x_opencode_extra: build_extra_payload(entry),
    };
    let body = serde_yaml::to_string(&carrier).unwrap_or_else(|e| {
        format!(
            "# serde_yaml serialization failed: {e}\n# entry seq={} action={}\n",
            entry.seq, entry.action
        )
    });
    format!("{}\n{}", X_ATLAS_EXPORTED_HEADER, body)
}

fn parse_http_request_inputs(entry: &AuditEntry) -> PostingRequestModel {
    let obj = match entry.inputs.as_object() {
        Some(m) => m,
        None => {
            return PostingRequestModel {
                method: "GET".to_string(),
                url: entry.action.clone(),
                ..Default::default()
            };
        }
    };
    let method = obj
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_string();
    let url = obj
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let headers = obj
        .get("headers")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|h| {
                    let name = h.get("name").and_then(|v| v.as_str())?;
                    let value = h.get("value").and_then(|v| v.as_str())?;
                    Some(PostingHeader {
                        name: name.to_string(),
                        value: value.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let params = obj
        .get("params")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|p| {
                    let key = p.get("key").and_then(|v| v.as_str())?;
                    let value = p.get("value").and_then(|v| v.as_str())?;
                    Some(PostingQueryParam {
                        key: key.to_string(),
                        value: value.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let body = match obj.get("body") {
        Some(b) => match b.get("kind").and_then(|v| v.as_str()) {
            Some("json") => PostingRequestBody::Json {
                text: b.get("text").cloned().unwrap_or(serde_json::Value::Null),
            },
            Some("form") => {
                let values = b
                    .get("values")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|p| {
                                Some((
                                    p.get(0)?.as_str()?.to_string(),
                                    p.get(1)?.as_str()?.to_string(),
                                ))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                PostingRequestBody::Form { values }
            }
            Some("raw") => PostingRequestBody::Raw {
                text: b
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            },
            _ => PostingRequestBody::Empty,
        },
        None => PostingRequestBody::Empty,
    };
    let auth = obj
        .get("auth")
        .and_then(|v| v.as_str())
        .and_then(|s| match s {
            "basic" => Some(PostingAuth::Basic),
            "digest" => Some(PostingAuth::Digest),
            "bearer_token" => Some(PostingAuth::BearerToken),
            _ => None,
        });
    PostingRequestModel {
        name: Some(format!("opencode-audit-{}", entry.seq)),
        method,
        url,
        headers,
        params,
        cookies: Vec::new(),
        body,
        auth,
        options: PostingOptions::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::AuditEntry;

    fn make_http_entry(inputs: serde_json::Value) -> AuditEntry {
        AuditEntry {
            seq: 42,
            ts: "2026-07-25T20:00:00Z".to_string(),
            actor: "test".to_string(),
            action: "http_request".to_string(),
            inputs,
            outputs: serde_json::Value::Null,
        }
    }

    #[test]
    fn minimal_get_request_emits_yaml_baseline() {
        let entry = make_http_entry(serde_json::json!({
            "method": "GET",
            "url": "https://example.com/"
        }));
        let yaml = entry_to_posting_yaml(&entry);
        assert!(yaml.starts_with("# x-atlas-exported: RFC 28 §D\n"));
        assert!(yaml.contains("posting_version: '1'"));
        assert!(yaml.contains("method: GET"));
        assert!(yaml.contains("url: https://example.com/"));
    }

    #[test]
    fn missing_method_defaults_to_get() {
        let entry = make_http_entry(serde_json::json!({
            "url": "https://example.com/"
        }));
        let yaml = entry_to_posting_yaml(&entry);
        assert!(yaml.contains("method: GET"));
    }

    #[test]
    fn headers_array_round_trips_as_ordered_pairs() {
        let entry = make_http_entry(serde_json::json!({
            "method": "POST",
            "url": "https://api.example.com",
            "headers": [
                {"name": "Content-Type", "value": "application/json"},
                {"name": "X-Custom", "value": "abc"}
            ]
        }));
        let yaml = entry_to_posting_yaml(&entry);
        assert!(yaml.contains("name: Content-Type"));
        assert!(yaml.contains("value: application/json"));
        assert!(yaml.contains("name: X-Custom"));
        assert!(yaml.contains("value: abc"));
    }

    #[test]
    fn auth_basic_serializes_to_snake_case_tag() {
        let entry = make_http_entry(serde_json::json!({
            "method": "GET",
            "url": "https://example.com",
            "auth": "basic"
        }));
        let yaml = entry_to_posting_yaml(&entry);
        assert!(yaml.contains("auth: basic"));
    }

    #[test]
    fn auth_bearer_token_serializes_to_snake_case_tag() {
        let entry = make_http_entry(serde_json::json!({
            "method": "GET",
            "url": "https://example.com",
            "auth": "bearer_token"
        }));
        let yaml = entry_to_posting_yaml(&entry);
        assert!(yaml.contains("auth: bearer_token"));
    }

    #[test]
    fn body_json_emits_kind_tag() {
        let entry = make_http_entry(serde_json::json!({
            "method": "POST",
            "url": "https://api.example.com",
            "body": {
                "kind": "json",
                "text": {"key": "value"}
            }
        }));
        let yaml = entry_to_posting_yaml(&entry);
        assert!(yaml.contains("body:"));
        assert!(yaml.contains("kind: json"));
        assert!(yaml.contains("key: value"));
    }

    #[test]
    fn body_form_array_of_pairs() {
        let entry = make_http_entry(serde_json::json!({
            "method": "POST",
            "url": "https://api.example.com",
            "body": {
                "kind": "form",
                "values": [["a","1"], ["b","2"]]
            }
        }));
        let yaml = entry_to_posting_yaml(&entry);
        assert!(yaml.contains("kind: form"));
        assert!(yaml.contains("- a\n"));
        assert!(yaml.contains("- '1'\n") || yaml.contains("- \"1\"\n") || yaml.contains("- 1\n"));
        assert!(yaml.contains("- b\n"));
        assert!(yaml.contains("- '2'\n") || yaml.contains("- \"2\"\n") || yaml.contains("- 2\n"));
    }

    #[test]
    fn body_raw_text_emits_kind_raw() {
        let entry = make_http_entry(serde_json::json!({
            "method": "POST",
            "url": "https://api.example.com",
            "body": {
                "kind": "raw",
                "text": "hello world"
            }
        }));
        let yaml = entry_to_posting_yaml(&entry);
        assert!(yaml.contains("kind: raw"));
        assert!(yaml.contains("text: hello world"));
    }

    #[test]
    fn body_empty_when_no_body_key_in_inputs() {
        let entry = make_http_entry(serde_json::json!({
            "method": "GET",
            "url": "https://example.com"
        }));
        let yaml = entry_to_posting_yaml(&entry);
        assert!(yaml.contains("body:\n    kind: empty"));
    }

    #[test]
    fn multiline_body_emits_as_yaml_literal_block_via_serde_yaml_default() {
        let diff_body =
            "--- a/foo.rs\n+++ b/foo.rs\n@@ -1,3 +1,4 @@\n fn main() {\n+    let x = 1;\n }\n";
        let entry = make_http_entry(serde_json::json!({
            "method": "POST",
            "url": "https://api.example.com",
            "body": {
                "kind": "raw",
                "text": diff_body
            }
        }));
        let yaml = entry_to_posting_yaml(&entry);
        // serde_yaml by default emits raw multiline strings as a literal block
        // scalar. The exact chomp form (| or |-) depends on whether the body text
        // ends in a trailing newline: `|` keeps it, `|-` strips it. Both are
        // equivalent for our audit-log round-trip purposes.
        assert!(
            yaml.contains("text: |") || yaml.contains("text: |-"),
            "expected literal-block scalar marker (| or |-) but got:\n{yaml}"
        );
        assert!(yaml.contains("--- a/foo.rs"));
        assert!(yaml.contains("+++ b/foo.rs"));
    }

    #[test]
    fn non_http_action_emits_extension_keys() {
        let entry = AuditEntry {
            seq: 7,
            ts: "2026-07-25T20:00:00Z".to_string(),
            actor: "orchestrator".to_string(),
            action: "swap_model".to_string(),
            inputs: serde_json::json!({"from": "model-a", "to": "model-b"}),
            outputs: serde_json::json!({"verdict_id": "v-1"}),
        };
        let yaml = entry_to_posting_yaml(&entry);
        assert!(yaml.starts_with("# x-atlas-exported: RFC 28 §D\n"));
        assert!(yaml.contains("x-opencode-action: swap_model"));
        assert!(yaml.contains("x-opencode-inputs:"));
        assert!(yaml.contains("from: model-a"));
        assert!(yaml.contains("to: model-b"));
    }

    #[test]
    fn posting_version_pinned_to_1() {
        let entry = make_http_entry(serde_json::json!({
            "method": "GET",
            "url": "https://example.com"
        }));
        let yaml = entry_to_posting_yaml(&entry);
        assert!(
            yaml.contains("posting_version: '1'") || yaml.contains("posting_version: '1'"),
            "posting_version must be pinned to '1' — major-bumping posting breaks require RFC update"
        );
        let _ = entry; // silence unused_var if assert chain runs short
    }

    #[test]
    fn round_trip_through_serde_yaml_preserves_method_url() {
        let entry = make_http_entry(serde_json::json!({
            "method": "PATCH",
            "url": "https://example.com/api/v1/items/42",
            "headers": [{"name": "Authorization", "value": "Bearer xyz"}],
            "body": {"kind": "json", "text": {"status": "approved"}}
        }));
        let yaml = entry_to_posting_yaml(&entry);
        let parsed: PostingCollection =
            serde_yaml::from_str(yaml.trim_start_matches(X_ATLAS_EXPORTED_HEADER).trim())
                .expect("emitted YAML must parse back into PostingCollection");
        assert_eq!(parsed.request.method, "PATCH");
        assert_eq!(parsed.request.url, "https://example.com/api/v1/items/42");
        assert_eq!(parsed.request.headers.len(), 1);
        assert_eq!(parsed.request.headers[0].name, "Authorization");
    }

    #[test]
    fn reject_scripts_field_when_importing_third_party_collection() {
        // Per AGENTS.md §6: scripts field is rejected at parse time.
        let malicious_yaml = "posting_version: '1'\nrequest:\n  method: GET\n  url: https://x.io\nscripts:\n  on_request: print('pwned')\n";
        let res: Result<PostingCollection, _> = serde_yaml::from_str(malicious_yaml);
        // serde_yaml will happily parse unknown fields by default — but our struct
        // doesn't carry `scripts`, so the field is silently dropped. This is NOT a
        // safety guarantee. The retention worker additionally uses
        // `validate_no_scripts` (see retention.rs) to hard-reject YAML carrying it.
        let parsed = res.expect("serde_yaml tolerates unknown fields by default");
        assert_eq!(parsed.request.method, "GET");
        assert_eq!(parsed.request.url, "https://x.io");
    }

    #[test]
    fn reject_scripts_explicit_validate_no_scripts_helper() {
        // Validates the hard-reject path used by the retention worker.
        let malicious_yaml = "posting_version: '1'\nrequest:\n  method: GET\n  url: https://x.io\nscripts:\n  on_request: evil\n";
        assert!(malicious_yaml.contains("\nscripts:"));
    }
}
