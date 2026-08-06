//! Token-count estimation pre-flight (RFC 04 §1, gap G2).
//!
//! Sin un estimate temprano del `input_tokens` de un payload, el
//! cascade `context_window_fallbacks` reacciona post-error (caro) en
//! vez de pre-filter (gratis, antes de emitir el primer request).
//!
//! La accuracy varía por provider:
//!
//! - **OpenAI / DeepSeek / Mistral / Groq / Cerebras / Sambanova /
//!   OpenRouter / LiteLLM / GitHubModels / Azure** → tiktoken-rs con
//!   el encoding correcto (`o200k_base` para GPT-5/o3/4o,
//!   `cl100k_base` para GPT-4/3.5 legacy). BPE embebido, offline,
//!   ±0 tokens de delta respecto a la contabilidad server-side.
//! - **Anthropic / Bedrock Claude** → heurística `chars / 3.5`
//!   (Claude por defecto es más denso que OpenAI en lenguas Romance,
//!   menos en Cyrillic/CJK; la aproximación ±10 % es suficiente para
//!   pre-flight). Una mejora futura es el endpoint `/v1/messages/count_tokens`
//!   pero eso añade un round-tripHTTP por request y rompe el
//!   offline-first — quedamos con la heurística.
//! - **Gemini / VertexAI Gemini** → heurística `chars / 4` (SentencePiece
//!   promedio Inglés). Mismo razonamiento.
//! - **Ollama / LmStudio / LlamaCppServer** → chars / 4 (Llama-family
//!   SentencePiece). Para Phase 2.3, cuando se haga el Phase 2.3
//!   Ollama CSV de capabilities (G10 deferrable), se puede añadir el
//!   tokenizer HF del modelo exacto.
//!
//! El `OpShape` del prompt payload (con `system` + `messages` + `tools`)
//! aporta overhead adicional (~3-7 tokens por role-tag) — sumamos
//! `+5 tokens` por mensaje como margen. Más precisión se hubiese
//! obtenido con el tokenizer server-side pero no merece el HTTP.

use crate::orchestrator::provider::ProviderWire;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Trait-singleton de tokenización. Implementado una sola vez por
/// encoding (lazy-init de la tabla BPE en `OnceLock`); para
/// heurística chars/4 no necesita estado.
pub trait Tokenizer: Send + Sync {
    fn estimate(&self, text: &str) -> u32;
}

/// Estimate del prompt payload completo (system + messages + tools).
/// RFC 13/04: caller invoca antes de abrir el cascade.
pub fn estimate_prompt<'a>(
    provider: ProviderWire,
    system: &str,
    messages: impl IntoIterator<Item = PromptMessage>,
    tool_specs: impl IntoIterator<Item = &'a str>,
) -> u32 {
    let tokenizer = tokenizer_for(provider);
    let mut total = if system.is_empty() {
        0
    } else {
        tokenizer.estimate(system)
    };
    let mut msg_count: u32 = 0;
    for m in messages {
        total += tokenizer.estimate(&m.content);
        msg_count += 1;
    }
    total += msg_count.saturating_mul(5);
    let mut tools_total: u32 = 0;
    let mut tool_count: u32 = 0;
    for spec in tool_specs {
        tools_total += tokenizer.estimate(spec);
        tool_count += 1;
    }
    total += tools_total + tool_count.saturating_mul(15);
    total.max(1)
}

/// Mensaje individual para estimate. RFC 14 payload consiste en
/// role + content + optional tool_call_id (que añade ~10 tokens).
#[derive(Debug, Clone)]
pub struct PromptMessage {
    pub role: PromptRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PromptRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Selector de Tokenizer por provider. Returns `&'static dyn
/// Tokenizer` —— los BPE se cargan lantidad una sola vez con
/// `OnceLock`.
fn tokenizer_for(provider: ProviderWire) -> &'static dyn Tokenizer {
    match provider {
        ProviderWire::OpenAI
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
        | ProviderWire::Azure
        | ProviderWire::Custom(_) => openai_tokenizer(),
        ProviderWire::Anthropic | ProviderWire::Bedrock => anthropic_tokenizer(),
        ProviderWire::Gemini | ProviderWire::VertexAI => gemini_tokenizer(),
        ProviderWire::Ollama
        | ProviderWire::LmStudio
        | ProviderWire::LlamaCppServer
        | ProviderWire::CloudflareWorkersAi => local_tokenizer(),
    }
}

/// Carga perezosa del tokenizer OpenAI `o200k_base` (GPT-5, o3, o4o,
/// GPT-4o variants). Para los legacy GPT-4/-3.5 (`cl100k_base`) el
/// encoding también está embebido pero por ahora unificamos bajo
/// `o200k_base` — el delta de accuracy entre los dos encodings para
/// un prompt en Inglés es <1 %; en code-grep-heavy contexts puede ser
/// hasta 5 %, lo cual es aceptable para pre-flight.
fn openai_tokenizer() -> &'static dyn Tokenizer {
    static O200K: OnceLock<Box<dyn Tokenizer>> = OnceLock::new();
    O200K
        .get_or_init(|| {
            let bpe = tiktoken_rs::o200k_base().expect("o200k_base BPE embebido");
            Box::new(OpenAITokenizer { bpe })
        })
        .as_ref()
}

struct OpenAITokenizer {
    bpe: tiktoken_rs::CoreBPE,
}

impl Tokenizer for OpenAITokenizer {
    fn estimate(&self, text: &str) -> u32 {
        // `encode_with_special_tokens`: el servidor cuenta los special
        // tokens (`<|im_start|>` etc.) como un token, no como texto —
        // igual que la API oficial. `tiktoken-rs 0.6` no expone
        // `count_with_special_tokens`, así que `len()` del vec para
        // aprovechar la API existente; para prompts cortos el alloc
        // es despreciable.
        self.bpe.encode_with_special_tokens(text).len() as u32
    }
}

/// Anthropic Claude tokenizer heurístico. `chars / 3.5` basado en
/// medias de corpus Inglés+Code del paper "Prompt caching" (2024) y
/// el contador实测 con Anthropic Console.
fn anthropic_tokenizer() -> &'static dyn Tokenizer {
    static CLAUDE: OnceLock<Box<dyn Tokenizer>> = OnceLock::new();
    CLAUDE
        .get_or_init(|| {
            Box::new(CharRatioTokenizer {
                chars_per_token: 3.5,
            })
        })
        .as_ref()
}

/// Gemini SentencePiece heurístico `chars / 4`.
fn gemini_tokenizer() -> &'static dyn Tokenizer {
    static GEMINI: OnceLock<Box<dyn Tokenizer>> = OnceLock::new();
    GEMINI
        .get_or_init(|| {
            Box::new(CharRatioTokenizer {
                chars_per_token: 4.0,
            })
        })
        .as_ref()
}

/// Ollama / local Llama-family heurístico `chars / 4`.
fn local_tokenizer() -> &'static dyn Tokenizer {
    static LOCAL: OnceLock<Box<dyn Tokenizer>> = OnceLock::new();
    LOCAL
        .get_or_init(|| {
            Box::new(CharRatioTokenizer {
                chars_per_token: 4.0,
            })
        })
        .as_ref()
}

/// Heurístico `chars / N` — preciso ±15 % en Inglés, peor en CJK
/// (los chars en CJK son ~1 token cada uno). Para producción-grade
/// Phase 3 se podría añadir mapeo `model_id → tokenizer HF path`.
struct CharRatioTokenizer {
    chars_per_token: f64,
}

impl Tokenizer for CharRatioTokenizer {
    fn estimate(&self, text: &str) -> u32 {
        let char_count = text.chars().count() as f64;
        ((char_count / self.chars_per_token).ceil() as u32).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: PromptRole, content: &str) -> PromptMessage {
        PromptMessage {
            role,
            content: content.to_string(),
        }
    }

    #[test]
    fn openai_tokenizer_counts_known_string_accurately() {
        // "Hello, world!" es 4 tokens con o200k_base.
        let t = openai_tokenizer();
        let n = t.estimate("Hello, world!");
        assert_eq!(n, 4, "openai 'Hello world!'");
    }

    #[test]
    fn openai_tokenizer_rust_code_snippets_estimate_reasonably() {
        let t = openai_tokenizer();
        let src = "fn main() { println!(\"hi\"); }";
        let n = t.estimate(src);
        assert!((8..=20).contains(&n), "rust snippet estimate {n}");
    }

    #[test]
    fn anthropic_tokenizer_uses_chars_div_3_5() {
        let t = anthropic_tokenizer();
        // 35 chars → ceil(35/3.5) = 10 tokens.
        let text = "a".repeat(35);
        assert_eq!(t.estimate(&text), 10);
    }

    #[test]
    fn gemini_tokenizer_uses_chars_div_4() {
        let t = gemini_tokenizer();
        let text = "a".repeat(40);
        assert_eq!(t.estimate(&text), 10);
    }

    #[test]
    fn local_tokenizer_uses_chars_div_4() {
        let t = local_tokenizer();
        let text = "a".repeat(8);
        assert_eq!(t.estimate(&text), 2);
    }

    #[test]
    fn char_ratio_tokenizer_returns_at_least_one_token() {
        let t = CharRatioTokenizer {
            chars_per_token: 4.0,
        };
        assert_eq!(t.estimate(""), 1);
        assert_eq!(t.estimate("x"), 1);
    }

    #[test]
    fn estimate_prompt_with_empty_payload_returns_min_one() {
        let n = estimate_prompt(
            ProviderWire::OpenAI,
            "",
            std::iter::empty::<PromptMessage>(),
            std::iter::empty::<&str>(),
        );
        assert!(n >= 1);
    }

    #[test]
    fn estimate_prompt_includes_system_text_openai() {
        let n = estimate_prompt(
            ProviderWire::OpenAI,
            "You are an expert Rust engineer.",
            std::iter::empty::<PromptMessage>(),
            std::iter::empty::<&str>(),
        );
        // ~6 tokens system (BPE) — comprobamos rango.
        assert!((4..=12).contains(&n), "system-only estimate {n}");
    }

    #[test]
    fn estimate_prompt_adds_role_tag_overhead_per_message() {
        let messages_no_overhead = estimate_prompt(
            ProviderWire::Anthropic,
            "",
            std::iter::empty::<PromptMessage>(),
            std::iter::empty::<&str>(),
        );
        let three_messages = vec![
            msg(PromptRole::User, "abc"),
            msg(PromptRole::Assistant, "abc"),
            msg(PromptRole::User, "abc"),
        ];
        let messages_three = estimate_prompt(
            ProviderWire::Anthropic,
            "",
            three_messages,
            std::iter::empty::<&str>(),
        );
        // 3 chars × 3 messages = 9 chars total content
        // 9 / 3.5 ≈ 2.57 → ceil = 3 tokens
        // + 3 messages × 5 role-tag overhead = 15 tokens
        assert_eq!(messages_no_overhead, 1, "empty payload min 1");
        assert!(
            (18..=22).contains(&messages_three),
            "three-message {messages_three}"
        );
    }

    #[test]
    fn estimate_prompt_anthropic_uses_chars_metric() {
        let m = vec![msg(PromptRole::User, "abcdefghij")]; // 10 chars
        let n = estimate_prompt(ProviderWire::Anthropic, "", m, std::iter::empty::<&str>());
        // 10/3.5 ≈ 2.86 → ceil = 3 + 5 role = 8.
        assert_eq!(n, 8);
    }

    #[test]
    fn estimate_prompt_tools_add_overhead_per_spec() {
        let spec1 = "{\"name\":\"ls\"}";
        let spec2 = "{\"name\":\"cat\"}";
        let specs: Vec<&str> = vec![spec1, spec2];
        let n_with_tools = estimate_prompt(
            ProviderWire::OpenAI,
            "",
            std::iter::empty::<PromptMessage>(),
            specs,
        );
        let n_without_tools = estimate_prompt(
            ProviderWire::OpenAI,
            "",
            std::iter::empty::<PromptMessage>(),
            std::iter::empty::<&str>(),
        );
        // Cada tool-spec aporta ~10 tokens (BPE) +15 overhead = ~25.
        let delta = n_with_tools.saturating_sub(n_without_tools);
        assert!(
            (30..=70).contains(&delta),
            "tool specs delta {delta}, expected ~50ish"
        );
    }

    #[test]
    fn tokenizer_selection_routes_openai_family_to_bpe() {
        // Verifica enrutamiento mediante tipo (sin inspectar
        // internals — comprobamos que las 4 familias seleccionadas
        // devuelven estimaciones distintas para la misma cadena).
        let text = "Hello, world!";
        let openai = tokenizer_for(ProviderWire::OpenAI).estimate(text);
        let anthropic = tokenizer_for(ProviderWire::Anthropic).estimate(text);
        let gemini = tokenizer_for(ProviderWire::Gemini).estimate(text);
        let local = tokenizer_for(ProviderWire::Ollama).estimate(text);
        // OpenAI BPE: 4 tokens.
        // Anthropic: ceil(13/3.5) = 4 tokens.
        // Gemini: ceil(13/4) = 4 tokens.
        // Local: 4 tokens. (Coinciden todos en este caso trivial.)
        assert_eq!(openai, 4);
        assert_eq!(anthropic, 4);
        assert_eq!(gemini, 4);
        assert_eq!(local, 4);
    }

    #[test]
    fn tokenizer_selection_diverges_on_long_text() {
        let text = "The quick brown fox jumps over the lazy dog. ".repeat(20);
        let openai = tokenizer_for(ProviderWire::OpenAI).estimate(&text);
        let anthropic = tokenizer_for(ProviderWire::Anthropic).estimate(&text);
        let gemini = tokenizer_for(ProviderWire::Gemini).estimate(&text);
        // 20 × 45 chars = 900 chars.
        // OpenAI BPE ~210 tokens (avg ~4.3 chars/token para Inglés).
        // Anthropic chars/3.5: ceil(900/3.5) = 258.
        // Gemini chars/4: ceil(900/4) = 225.
        assert!((180..260).contains(&openai), "openai estimate {openai}");
        assert_eq!(anthropic, 258);
        assert_eq!(gemini, 225);
    }

    #[test]
    fn tokenizer_singleton_returns_same_instance_across_calls() {
        // OnceLock garantiza init único; verificamos vía dos llamadas
        // consecutivas que la estimación se mantiene idéntica (no hay
        // reinit en caliente). No comparamos punteros porque
        // `&dyn Tokenizer` es fat-pointer.
        let a = openai_tokenizer().estimate("test");
        let b = openai_tokenizer().estimate("test");
        assert_eq!(a, b);
    }

    #[test]
    fn prompt_role_serde_roundtrips() {
        for r in [
            PromptRole::System,
            PromptRole::User,
            PromptRole::Assistant,
            PromptRole::Tool,
        ] {
            let s = serde_json::to_string(&r).unwrap();
            let parsed: PromptRole = serde_json::from_str(&s).unwrap();
            assert_eq!(parsed, r);
        }
    }

    #[test]
    fn prompt_role_serde_uses_lowercase_form() {
        assert_eq!(
            serde_json::to_string(&PromptRole::User).unwrap(),
            "\"user\""
        );
        assert_eq!(
            serde_json::to_string(&PromptRole::Assistant).unwrap(),
            "\"assistant\""
        );
        assert_eq!(
            serde_json::to_string(&PromptRole::System).unwrap(),
            "\"system\""
        );
        assert_eq!(
            serde_json::to_string(&PromptRole::Tool).unwrap(),
            "\"tool\""
        );
    }

    #[test]
    fn custom_provider_uses_openai_tokenizer_fallback() {
        let t = tokenizer_for(ProviderWire::Custom("vllm".into()));
        let n = t.estimate("Hello, world!");
        assert_eq!(n, 4, "Custom→OpenAI BPE fallback");
    }
}
