//! Prompt-cache control markers + cache-read token accounting (RFC 04 §1, gap G1).
//!
//! Sin tracking del `cache_read_input_tokens`, el feedback loop de
//! Phase 2 reporta modelos Anthropic/OpenAI 3-5× más caros de lo
//! real → el affinity classifier aprende mal (`was_correct` queda
//! sesgado contra caching winners). Esta capa expone dos
//! operaciones:
//!
//! - **Marker injection**: antes de dispatchar el prompt al
//!   provider, se insertan breakpoints de cache en los content blocks
//!   estables (system/messages/tools) según el dialecto del
//!   provider. En Anthropic los markers se colocan con
//!   `cache_control: { type: "ephemeral", ttl }`; en OpenAI-5.6+
//!   con `prompt_cache_breakpoint: { mode: "explicit" }` y
//!   `prompt_cache_options.mode`. Otros providers (Gemini, DeepSeek
//!   vía OpenRouter) cachean automáticamente sin markers — la
//!   función es no-op para ellos.
//!
//! - **Cache-read extraction**: tras el response, parsear el campo
//!   de_usage del dialecto para obtener `cache_read_input_tokens`
//!   (Anthropic) / `cached_tokens` (OpenAI) / `cache_read` (Gemini).
//!   El feedback loop los resta del cost computation.
//!
//! Decisiones de diseño:
//!
//! - **Marker**: en Anthropic, marcar los últimos blocks cacheables
//!   es lo más efectivo (cachea hasta ese punto); se inyecta
//!   `cache_control` en el último `content` del system y en cada
//!   message hasta llegar a 4 breakpoints (límite Anthropic). En
//!   OpenAI, basta un breakpoint en el último content block
//!   estático; `prompt_cache_options.mode = "explicit"` desactiva
//!   el implicit para evitar cache writes en changing suffixes.
//! - **TTL**: Anthropic `"5m"` default, `"1h"` opt-in. OpenAI
//!   GPT-5.6+ sólo soporta `"30m"`. Gemini automático sin TTL.
//! - **No-op providers**: Bedrock Claude hereda Anthropic dialecto
//!   (`cache_control` sigue funcionando vía `input` field). Local
//!   (Ollama/LlamaCpp/LmStudio) no cachean — no-op.
//!
//! El extractor devuelve `Option<u32>`: algunos providers no
//! informan cache_read en cada response (Gemini-API antiguas);
//! `None` es distinto de `Some(0)` — `Some(0)` indica cache miss
//! confirmado, `None` indica "no informativo" (no restar del cost).

use crate::orchestrator::provider::ProviderWire;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContentKind {
    Array,
    String,
    Other,
}

/// TTL del cache ephemeral. Header del marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheTtl {
    /// Anthropic 5 minutos, OpenAI GPT-5.6 30 minutos. Default
    /// recomendado — cubre lengt típico de sesión interactiva
    /// sin pagar excessive cache writes.
    Short,
    /// Anthropic 1 hora. Para jobs largos (autoresearch, big batch
    /// eval). El costo write es ~12.5× después de 5 min inactivity.
    Long,
}

impl CacheTtl {
    pub fn anthropic_str(&self) -> &'static str {
        match self {
            CacheTtl::Short => "5m",
            CacheTtl::Long => "1h",
        }
    }
    pub fn openai_str(&self) -> &'static str {
        // OpenAI GPT-5.6+ sólo soporta "30m". Mapeamos ambos a eso.
        "30m"
    }
}

/// Política de cache control. Per-request — sobreescribe defaults
/// del registry si el caller lo pide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CachePolicy {
    /// Modo default — provider coloca el marker implícito. En
    /// Anthropic cachea system + tools automáticamente; en OpenAI
    /// coloca un breakpoint en el último message. Útil cuando el
    /// caller no sabe qué es estable.
    Implicit,
    /// Modo controlado — sólo markers explícitos en content blocks
    /// son cacheables. Previene cache writes de changing suffixes
    /// (caro en OpenAI 1.25×). Caller debe inyectar breakpoints
    /// en el payload (ver `inject_breakpoints`).
    Explicit,
}

/// Inserta breakpoints de cache en el payload según el dialecto
/// del provider y la política. Mutates `payload` in-place.
///
/// **`payload` shape esperado**:
/// - Anthropic / Bedrock Claude:
///   ```json
///   { "system": "...", "messages": [{ "role": ..., "content": ... }],
///     "tools": [...] }
///   ```
/// - OpenAI Chat Completions / DeepSeek / Mistral / OpenRouter:
///   ```json
///   { "messages": [{ "role", "content": [...] }] }
///   ```
/// - Gemini / VertexAI:
///   ```json
///   { "contents": [{ "role", "parts": [...] }], "systemInstruction": ... }
///   ```
///
/// Para providers outsiders (Ollama, LlamaCppServer), no-op.
///
/// `breakpoint_count` ajusta el máximo número de markers a insertar.
/// Anthropic permite hasta 4 simultáneos; OpenAI GPT-5.6+ 4 new
/// writes por request. Por defecto `2` (balance costo/eficiencia).
pub fn inject_breakpoints(
    payload: &mut serde_json::Value,
    provider: ProviderWire,
    policy: CachePolicy,
    ttl: CacheTtl,
    breakpoint_count: u8,
) {
    if breakpoint_count == 0 {
        return;
    }
    match provider {
        ProviderWire::Anthropic | ProviderWire::Bedrock => {
            inject_anthropic(payload, ttl, breakpoint_count);
        }
        ProviderWire::OpenAI
        | ProviderWire::Azure
        | ProviderWire::DeepSeek
        | ProviderWire::Mistral
        | ProviderWire::Groq
        | ProviderWire::Cerebras
        | ProviderWire::Sambanova
        | ProviderWire::OpenRouter
        | ProviderWire::LiteLLM
        | ProviderWire::GitHubModels
        | ProviderWire::NvidiaNim
        | ProviderWire::HuggingFaceInference
        | ProviderWire::Custom(_) => {
            inject_openai_compat(payload, policy, ttl, breakpoint_count);
        }
        ProviderWire::Gemini | ProviderWire::VertexAI => {
            // Gemini caching automático. No markers en el request;
            // se controla vía `cachedContent` referencia (no soportado
            // por defecto aquí — anotado como Future Work G20).
        }
        ProviderWire::Ollama
        | ProviderWire::LmStudio
        | ProviderWire::LlamaCppServer
        | ProviderWire::CloudflareWorkersAi => {
            // Sin cache ephemeral; sólo max_keepalive del backend.
            // No-op para marker injection.
        }
    }
}

/// Anthropic: añade `cache_control: { type: "ephemeral", ttl }` en
/// los últimos `count` content blocks cacheables. Intenta system,
/// luego tools, luego messages (en ese orden de prioridad, para
/// maximizar el prefijo cacheado).
fn inject_anthropic(payload: &mut serde_json::Value, ttl: CacheTtl, count: u8) {
    let mut remaining = count;
    // system puede venir como string o como array de content.
    if let Some(sys) = payload.get_mut("system") {
        if let Some(arr) = sys.as_array_mut() {
            // último block del system es un buen candidate.
            if let Some(last) = arr.last_mut() {
                insert_anthropic_marker(last, ttl);
                remaining = remaining.saturating_sub(1);
            }
        } else if let Some(text) = sys.as_str() {
            let mut block = serde_json::Value::Object(serde_json::Map::new());
            if let Some(obj) = block.as_object_mut() {
                obj.insert("type".to_string(), serde_json::Value::String("text".into()));
                obj.insert(
                    "text".to_string(),
                    serde_json::Value::String(text.to_string()),
                );
            }
            insert_anthropic_marker(&mut block, ttl);
            *sys = serde_json::Value::Array(vec![block]);
            remaining = remaining.saturating_sub(1);
        }
    }
    if remaining == 0 {
        return;
    }
    // messages: añade marker al último content del último message
    // (el último message es el más dinámico, generalmente NO se
    // cachea — mejor target es el penúltimo).
    if let Some(messages) = payload.get_mut("messages").and_then(|v| v.as_array_mut()) {
        // Penúltimo message si hay (que es más estable), si no último.
        if let Some(target) = messages
            .len()
            .checked_sub(2)
            .and_then(|i| messages.get_mut(i))
        {
            // Evita borrow-checker issues: inspecciona tipos antes de
            // tomar el borrow mutable.
            let content_kind = target
                .get("content")
                .map(|v| match v {
                    serde_json::Value::Array(_) => ContentKind::Array,
                    serde_json::Value::String(_) => ContentKind::String,
                    _ => ContentKind::Other,
                })
                .unwrap_or(ContentKind::Other);
            match content_kind {
                ContentKind::Array => {
                    if let Some(last_block) = target
                        .get_mut("content")
                        .and_then(|v| v.as_array_mut())
                        .and_then(|a| a.last_mut())
                    {
                        insert_anthropic_marker(last_block, ttl);
                        remaining = remaining.saturating_sub(1);
                    }
                }
                ContentKind::String => {
                    let content_str = target
                        .get("content")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    if let Some(s) = content_str {
                        let mut block = serde_json::Value::Object(serde_json::Map::new());
                        if let Some(obj) = block.as_object_mut() {
                            obj.insert(
                                "type".to_string(),
                                serde_json::Value::String("text".into()),
                            );
                            obj.insert("text".to_string(), serde_json::Value::String(s));
                        }
                        insert_anthropic_marker(&mut block, ttl);
                        target["content"] = serde_json::Value::Array(vec![block]);
                        remaining = remaining.saturating_sub(1);
                    }
                }
                ContentKind::Other => {
                    // No es array ni string — no podemos insert.
                }
            }
        }
    }
    if remaining == 0 {
        return;
    }
    // tools: último tool spec.
    if let Some(tools) = payload.get_mut("tools").and_then(|v| v.as_array_mut()) {
        if let Some(last_tool) = tools.last_mut() {
            insert_anthropic_marker(last_tool, ttl);
        }
    }
}

/// Añade un `cache_control` field en un block JSON. Idempotente:
/// si ya tiene cache_control, no sobreescribe.
fn insert_anthropic_marker(block: &mut serde_json::Value, ttl: CacheTtl) {
    if let Some(obj) = block.as_object_mut() {
        if obj.contains_key("cache_control") {
            return;
        }
        let mut marker = serde_json::Map::new();
        marker.insert(
            "type".to_string(),
            serde_json::Value::String("ephemeral".into()),
        );
        marker.insert(
            "ttl".to_string(),
            serde_json::Value::String(ttl.anthropic_str().into()),
        );
        obj.insert(
            "cache_control".to_string(),
            serde_json::Value::Object(marker),
        );
    }
}

/// OpenAI-compat (Chat Completions): añade
/// `prompt_cache_breakpoint: { mode: "explicit" }` en el último
/// content block del último `messages` estático. Si `policy =
/// explicit`, también añade `prompt_cache_options.mode = "explicit"`
/// al top-level para deshabilitar el implicit breakpoint.
fn inject_openai_compat(
    payload: &mut serde_json::Value,
    policy: CachePolicy,
    _ttl: CacheTtl,
    count: u8,
) {
    if count == 0 {
        return;
    }
    // 1) Top-level prompt_cache_options en explicit mode.
    if policy == CachePolicy::Explicit {
        let mut opts = serde_json::Map::new();
        opts.insert(
            "mode".to_string(),
            serde_json::Value::String("explicit".into()),
        );
        payload["prompt_cache_options"] = serde_json::Value::Object(opts);
    }
    // 2) Marker en el último block del penúltimo message.
    if let Some(messages) = payload.get_mut("messages").and_then(|v| v.as_array_mut()) {
        // Si los content-blocks NO son objetos (sólo string),
        // marker no aplica — fallback dejar el string.
        if let Some(target) = messages
            .len()
            .checked_sub(2)
            .and_then(|i| messages.get_mut(i))
        {
            if let Some(content) = target.get_mut("content").and_then(|v| v.as_array_mut()) {
                let mut inserts_left = count;
                for block in content.iter_mut().rev() {
                    if let Some(obj) = block.as_object_mut() {
                        if !obj.contains_key("prompt_cache_breakpoint") {
                            let mut m = serde_json::Map::new();
                            m.insert(
                                "mode".to_string(),
                                serde_json::Value::String("explicit".into()),
                            );
                            obj.insert(
                                "prompt_cache_breakpoint".to_string(),
                                serde_json::Value::Object(m),
                            );
                            inserts_left = inserts_left.saturating_sub(1);
                            if inserts_left == 0 {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Extrae del response `Usage` el número de tokens servidos desde
/// el prompt cache del provider. `Some(0)` = cache miss confirmado;
/// `None` = no informativo (provider no reporta cache).
pub fn extract_cache_read(response: &serde_json::Value, provider: ProviderWire) -> Option<u32> {
    let usage = response.get("usage")?;
    match provider {
        ProviderWire::Anthropic | ProviderWire::Bedrock => usage
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        ProviderWire::OpenAI
        | ProviderWire::Azure
        | ProviderWire::DeepSeek
        | ProviderWire::Mistral
        | ProviderWire::Groq
        | ProviderWire::Cerebras
        | ProviderWire::Sambanova
        | ProviderWire::OpenRouter
        | ProviderWire::LiteLLM
        | ProviderWire::GitHubModels
        | ProviderWire::NvidiaNim
        | ProviderWire::HuggingFaceInference
        | ProviderWire::Custom(_) => usage
            .get("prompt_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        ProviderWire::Gemini | ProviderWire::VertexAI => usage
            .get("cache_read_token_count")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        ProviderWire::Ollama
        | ProviderWire::LmStudio
        | ProviderWire::LlamaCppServer
        | ProviderWire::CloudflareWorkersAi => None,
    }
}

/// Extrae cache_creation tokens (writeCount). Útil para reporte
/// de coste del primer request que llena el cache.
pub fn extract_cache_creation(response: &serde_json::Value, provider: ProviderWire) -> Option<u32> {
    let usage = response.get("usage")?;
    match provider {
        ProviderWire::Anthropic | ProviderWire::Bedrock => usage
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        ProviderWire::OpenAI
        | ProviderWire::Azure
        | ProviderWire::DeepSeek
        | ProviderWire::Mistral
        | ProviderWire::Groq
        | ProviderWire::Cerebras
        | ProviderWire::Sambanova
        | ProviderWire::OpenRouter
        | ProviderWire::LiteLLM
        | ProviderWire::GitHubModels
        | ProviderWire::NvidiaNim
        | ProviderWire::HuggingFaceInference
        | ProviderWire::Custom(_) => usage
            .get("prompt_tokens_details")
            .and_then(|d| d.get("cache_write_tokens"))
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        ProviderWire::Gemini | ProviderWire::VertexAI => usage
            .get("cache_write_token_count")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        ProviderWire::Ollama
        | ProviderWire::LmStudio
        | ProviderWire::LlamaCppServer
        | ProviderWire::CloudflareWorkersAi => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cache_ttl_anthropic_str_maps_correctly() {
        assert_eq!(CacheTtl::Short.anthropic_str(), "5m");
        assert_eq!(CacheTtl::Long.anthropic_str(), "1h");
    }

    #[test]
    fn cache_ttl_openai_str_is_always_30m() {
        assert_eq!(CacheTtl::Short.openai_str(), "30m");
        assert_eq!(CacheTtl::Long.openai_str(), "30m");
    }

    #[test]
    fn cache_policy_serde_roundtrips() {
        let s = serde_json::to_string(&CachePolicy::Explicit).unwrap();
        assert_eq!(s, "\"explicit\"");
        let p: CachePolicy = serde_json::from_str("\"implicit\"").unwrap();
        assert_eq!(p, CachePolicy::Implicit);
    }

    #[test]
    fn inject_anthropic_system_string_converts_to_array_with_marker() {
        let mut payload = json!({
            "system": "You are a Rust engineer.",
            "messages": [{"role":"user","content":"hi"}]
        });
        inject_breakpoints(
            &mut payload,
            ProviderWire::Anthropic,
            CachePolicy::Implicit,
            CacheTtl::Short,
            1,
        );
        let system = payload.get("system").unwrap();
        assert!(system.is_array(), "system debe ser array tras inject");
        let first = system.as_array().unwrap()[0].as_object().unwrap();
        assert_eq!(first.get("type").and_then(|v| v.as_str()), Some("text"));
        let cc = first.get("cache_control").unwrap().as_object().unwrap();
        assert_eq!(cc.get("type").and_then(|v| v.as_str()), Some("ephemeral"));
        assert_eq!(cc.get("ttl").and_then(|v| v.as_str()), Some("5m"));
    }

    #[test]
    fn inject_anthropic_system_array_adds_marker_to_last_block() {
        let mut payload = json!({
            "system": [
                {"type":"text","text":"You are..."},
                {"type":"text","text":"More instructions"}
            ],
            "messages": [{"role":"user","content":"hi"}]
        });
        inject_breakpoints(
            &mut payload,
            ProviderWire::Anthropic,
            CachePolicy::Implicit,
            CacheTtl::Long,
            1,
        );
        let arr = payload["system"].as_array().unwrap();
        let last = arr[1].as_object().unwrap();
        assert!(last.contains_key("cache_control"));
        assert_eq!(
            last["cache_control"]["ttl"].as_str(),
            Some("1h"),
            "Long TTL maps to 1h"
        );
        let first = arr[0].as_object().unwrap();
        assert!(
            !first.contains_key("cache_control"),
            "sólo el último recibe"
        );
    }

    #[test]
    fn inject_anthropic_idempotent_does_not_double_insert() {
        let mut payload = json!({
            "system": [{"type":"text","text":"hi","cache_control":{"type":"ephemeral","ttl":"5m"}}],
            "messages": []
        });
        inject_breakpoints(
            &mut payload,
            ProviderWire::Anthropic,
            CachePolicy::Implicit,
            CacheTtl::Long,
            3,
        );
        let first_block = &payload["system"][0];
        let cc = first_block["cache_control"].as_object().unwrap();
        // No debería sobreescribir el ttl original.
        assert_eq!(cc["ttl"].as_str(), Some("5m"));
    }

    #[test]
    fn inject_anthropic_messages_string_content_converts_to_array() {
        let mut payload = json!({
            "system": "sys",
            "messages": [
                {"role":"user","content":"stable static msg"},
                {"role":"user","content":"changing last"}
            ]
        });
        inject_breakpoints(
            &mut payload,
            ProviderWire::Anthropic,
            CachePolicy::Implicit,
            CacheTtl::Short,
            2,
        );
        let msgs = payload["messages"].as_array().unwrap();
        let penultimo = &msgs[0];
        let content = penultimo["content"].as_array().unwrap();
        let block = content[0].as_object().unwrap();
        assert!(
            block.contains_key("cache_control"),
            "penúltimo recibe marker con count=2"
        );
    }

    #[test]
    fn inject_anthropic_zero_count_no_op() {
        let mut payload = json!({"system":[{"type":"text","text":"hola"}]});
        let orig = payload.clone();
        inject_breakpoints(
            &mut payload,
            ProviderWire::Anthropic,
            CachePolicy::Implicit,
            CacheTtl::Short,
            0,
        );
        assert_eq!(payload, orig, "count=0 → no-op");
    }

    #[test]
    fn inject_openai_explicit_adds_top_level_prompt_cache_options() {
        let mut payload = json!({
            "messages": [
                {"role":"system","content":[{"type":"text","text":"You are..."}]},
                {"role":"user","content":"hi"}
            ]
        });
        inject_breakpoints(
            &mut payload,
            ProviderWire::OpenAI,
            CachePolicy::Explicit,
            CacheTtl::Short,
            1,
        );
        let opts = payload.get("prompt_cache_options").unwrap();
        assert_eq!(opts["mode"].as_str(), Some("explicit"));
    }

    #[test]
    fn inject_openai_implicit_does_not_set_top_level_options() {
        let mut payload = json!({
            "messages": [
                {"role":"system","content":[{"type":"text","text":"You are..."}]},
                {"role":"user","content":"hi"}
            ]
        });
        inject_breakpoints(
            &mut payload,
            ProviderWire::OpenAI,
            CachePolicy::Implicit,
            CacheTtl::Short,
            1,
        );
        assert!(
            !payload
                .as_object()
                .unwrap()
                .contains_key("prompt_cache_options"),
            "Implicit no añade top-level"
        );
        // Aún así añade marker en el penúltimo message:
        let sys = &payload["messages"][0];
        let block = &sys["content"][0];
        assert!(block
            .as_object()
            .unwrap()
            .contains_key("prompt_cache_breakpoint"));
    }

    #[test]
    fn inject_openai_marker_on_penultimate_content_block() {
        let mut payload = json!({
            "messages": [
                {"role":"system","content":[{"type":"text","text":"s1"},{"type":"text","text":"s2"}]},
                {"role":"user","content":[{"type":"text","text":"u1"}]}
            ]
        });
        inject_breakpoints(
            &mut payload,
            ProviderWire::OpenAI,
            CachePolicy::Explicit,
            CacheTtl::Short,
            1,
        );
        let msgs = payload["messages"].as_array().unwrap();
        // penúltimo es el primero aquí (length 2).
        let system_blocks = msgs[0]["content"].as_array().unwrap();
        // último block del penúltimo message recibe el marker.
        let last_block = system_blocks.last().unwrap().as_object().unwrap();
        assert!(last_block.contains_key("prompt_cache_breakpoint"));
    }

    #[test]
    fn inject_openai_count_caps_on_2_blocks_for_multiple() {
        let mut payload = json!({
            "messages": [
                {"role":"system","content":[
                    {"type":"text","text":"a"},
                    {"type":"text","text":"b"},
                    {"type":"text","text":"c"}
                ]},
                {"role":"user","content":"hi"}
            ]
        });
        inject_breakpoints(
            &mut payload,
            ProviderWire::OpenAI,
            CachePolicy::Explicit,
            CacheTtl::Short,
            2,
        );
        let blocks = payload["messages"][0]["content"].as_array().unwrap();
        let mut marker_count = 0;
        for b in blocks {
            if b.as_object()
                .unwrap()
                .contains_key("prompt_cache_breakpoint")
            {
                marker_count += 1;
            }
        }
        assert_eq!(marker_count, 2, "count=2 → 2 markers");
    }

    #[test]
    fn inject_gemini_is_no_op() {
        let mut payload = json!({"contents":[{"role":"user","parts":[{"text":"hola"}]}]});
        let orig = payload.clone();
        inject_breakpoints(
            &mut payload,
            ProviderWire::Gemini,
            CachePolicy::Explicit,
            CacheTtl::Long,
            5,
        );
        assert_eq!(payload, orig, "Gemini caching automático — no-op");
    }

    #[test]
    fn inject_local_providers_no_op() {
        let mut payload = json!({"messages":[{"role":"user","content":"hi"}]});
        let orig = payload.clone();
        for p in [
            ProviderWire::Ollama,
            ProviderWire::LmStudio,
            ProviderWire::LlamaCppServer,
        ] {
            inject_breakpoints(
                &mut payload,
                p.clone(),
                CachePolicy::Implicit,
                CacheTtl::Short,
                3,
            );
            assert_eq!(payload, orig, "{p:?} debe ser no-op");
        }
    }

    #[test]
    fn extract_cache_read_anthropic_reads_field() {
        let resp = json!({
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "cache_read_input_tokens": 80
            }
        });
        let n = extract_cache_read(&resp, ProviderWire::Anthropic);
        assert_eq!(n, Some(80));
    }

    #[test]
    fn extract_cache_read_anthropic_missing_returns_none() {
        let resp = json!({"usage":{"input_tokens":10}});
        assert_eq!(extract_cache_read(&resp, ProviderWire::Anthropic), None);
    }

    #[test]
    fn extract_cache_read_openai_digs_into_prompt_tokens_details() {
        let resp = json!({
            "usage": {
                "prompt_tokens": 2006,
                "completion_tokens": 300,
                "prompt_tokens_details": {"cached_tokens": 1920}
            }
        });
        assert_eq!(extract_cache_read(&resp, ProviderWire::OpenAI), Some(1920));
    }

    #[test]
    fn extract_cache_read_openai_missing_field_returns_none() {
        let resp = json!({"usage":{"prompt_tokens":10}});
        assert_eq!(extract_cache_read(&resp, ProviderWire::OpenAI), None);
    }

    #[test]
    fn extract_cache_read_gemini_reads_cache_read_token_count() {
        let resp = json!({
            "usage": {"token_count": 100, "cache_read_token_count": 90}
        });
        assert_eq!(extract_cache_read(&resp, ProviderWire::Gemini), Some(90));
    }

    #[test]
    fn extract_cache_read_local_returns_none() {
        let resp = json!({"usage":{"tokens":10}});
        for p in [
            ProviderWire::Ollama,
            ProviderWire::LmStudio,
            ProviderWire::LlamaCppServer,
        ] {
            assert_eq!(extract_cache_read(&resp, p), None);
        }
    }

    #[test]
    fn extract_cache_creation_anthropic_reads_field() {
        let resp = json!({
            "usage": {"cache_creation_input_tokens": 30}
        });
        assert_eq!(
            extract_cache_creation(&resp, ProviderWire::Anthropic),
            Some(30)
        );
    }

    #[test]
    fn extract_cache_creation_openai_reads_cache_write_tokens() {
        let resp = json!({
            "usage": {"prompt_tokens_details": {"cache_write_tokens": 512}}
        });
        assert_eq!(
            extract_cache_creation(&resp, ProviderWire::OpenAI),
            Some(512)
        );
    }

    #[test]
    fn extract_cache_creation_gemini_reads_cache_write_token_count() {
        let resp = json!({"usage":{"cache_write_token_count":700}});
        assert_eq!(
            extract_cache_creation(&resp, ProviderWire::Gemini),
            Some(700)
        );
    }

    #[test]
    fn extract_cache_no_usage_object_returns_none() {
        let resp = json!({"output":"yes"});
        assert_eq!(extract_cache_read(&resp, ProviderWire::OpenAI), None);
        assert_eq!(extract_cache_creation(&resp, ProviderWire::Anthropic), None);
    }

    #[test]
    fn extract_custom_provider_uses_openai_dialect() {
        let resp = json!({
            "usage": {"prompt_tokens_details": {"cached_tokens": 5}}
        });
        assert_eq!(
            extract_cache_read(&resp, ProviderWire::Custom("vllm".into())),
            Some(5)
        );
    }
}
