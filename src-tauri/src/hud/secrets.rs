// Atlas OS — HUD API-key settings (RFC 25 §3.10; RFC 65 §10).
//
// The in-app surface over the OS keychain (`crate::secrets`): list the known
// provider slots, store a key, delete one. This is the product path (the
// desktop Settings screen), mirroring how opencode/Grok/Cursor let you paste
// a provider key in the UI.
//
// Security: the value is **never** returned and **never** logged — the read
// side exposes only a `present` boolean. The HUD binds loopback by default;
// when exposed remotely (`atlas serve --host …`) it must sit behind the
// authenticated tunnel/bearer (RFC 18), same as every other HUD route.

use axum::extract::Path;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

/// Provider key slots offered in the UI — the `api_key_env` names common to
/// the model registry. Any other name can still be posted.
const CATALOG: &[&str] = &[
    "OPENCODE_GO_KEY",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "OPENROUTER_API_KEY",
    "GEMINI_API_KEY",
    "GROQ_API_KEY",
    "FIRECRAWL_API_KEY",
];

#[derive(Debug, Deserialize)]
pub struct SetSecretBody {
    pub account: String,
    pub value: String,
}

fn bad_request(message: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message })))
}

/// `GET /hud/secrets` — the catalog with a `present` flag per slot. Never
/// returns a value.
pub async fn get_secrets() -> Json<Value> {
    let slots: Vec<Value> = CATALOG
        .iter()
        .map(|account| {
            let present = matches!(crate::secrets::get(account), Ok(Some(_)));
            json!({ "account": account, "present": present })
        })
        .collect();
    Json(json!({ "service": crate::secrets::SERVICE, "slots": slots }))
}

/// `POST /hud/secrets` — store a key in the OS keychain.
pub async fn post_secret(
    Json(body): Json<SetSecretBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let account = body.account.trim();
    if account.is_empty() || body.value.trim().is_empty() {
        return Err(bad_request("account and value are required"));
    }
    match crate::secrets::set(account, &body.value) {
        Ok(()) => Ok(Json(json!({ "stored": true, "account": account }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}

/// `DELETE /hud/secrets/:account` — remove a key from the OS keychain.
pub async fn delete_secret(
    Path(account): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let account = account.trim();
    if account.is_empty() {
        return Err(bad_request("account is required"));
    }
    match crate::secrets::delete(account) {
        Ok(deleted) => Ok(Json(json!({ "deleted": deleted, "account": account }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_lists_the_catalog_with_present_flags_and_no_values() {
        let Json(v) = get_secrets().await;
        assert_eq!(v["service"], crate::secrets::SERVICE);
        let slots = v["slots"].as_array().unwrap();
        assert_eq!(slots.len(), CATALOG.len());
        assert!(slots.iter().any(|s| s["account"] == "OPENCODE_GO_KEY"));
        // No field carries a secret value.
        assert!(slots.iter().all(|s| s.get("value").is_none()));
        assert!(slots.iter().all(|s| s["present"].is_boolean()));
    }

    #[tokio::test]
    async fn post_rejects_blank_input_before_touching_the_store() {
        let err = post_secret(Json(SetSecretBody {
            account: "OPENCODE_GO_KEY".into(),
            value: "   ".into(),
        }))
        .await
        .unwrap_err();
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        let err = post_secret(Json(SetSecretBody {
            account: "  ".into(),
            value: "x".into(),
        }))
        .await
        .unwrap_err();
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }
}
