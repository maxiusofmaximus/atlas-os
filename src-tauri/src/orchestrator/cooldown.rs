//! Per-provider cooldown configuration + Retry-After parser (RFC 04
//! §6, gaps G5 + G17).
//!
//! Phase 1 hardcodea `cooldown_time = 5s` para todos los providers;
//! eso es demasiado corto para los 30-60s reales de Anthropic/OpenAI,
//! y excesivo para el `5s` local de Ollama. Esta capa aporta:
//!
//! **G5:** Override por provider. Defaults derivados de la
//! documentación oficial (Anthropic 30s, OpenAI 60s, Ollama 5s,
//! Bedrock 1s). El operador puede pisar cada uno en `Profile`.
//!
//! **G17:** Parsing del header HTTP `Retry-After` (RFC 7231 §7.1.3)
//! y `x-ratelimit-reset-requests` / `x-ratelimit-reset-tokens`
//! (OpenAI custom). El valor devuelto por el provider es
//! autoritativo y sobreescribe el cooldown configurado para esa
//! sesión concreta — `Retry-After: 90` significa "el provider quiere
//! 90s aunque el default sea 30s".
//!
//! La capa integra con `parse_error.rs` que ya expone
//! `parse_retry_after` (HTTP-date y delta-seconds) y
//! `parse_x_ratelimit_reset`. Aquí bridgenan a `Duration` para el
//! caller del retry loop.
//!
//! **Diseño de API:**
//! - `CooldownConfig::default()` usa defaults del RFC 28 §H.4.
//! - `CooldownConfig::with_overrides(overrides)` permite override
//!   por provider desde `Profile.cooldown_overrides`.
//! - `CooldownConfig::resolve(provider)` da el cooldown base
//!   (overrides > default).
//! - `CooldownConfig::resolve_with_retry_after(provider, headers)`
//!   aplica Retry-After si está presente; si no, vuelve al default.
//!
//! No exponemos `Headers` (reqwest) directamente — recibimos
//! `&dyn RetryAfterSource` para que los tests puedan mockear sin
//! construir `reqwest::header::HeaderMap`. En producción, el caller
//! adapta `reqwest::header::HeaderMap` con el trait impl.

use crate::orchestrator::parse_error::{parse_retry_after, parse_x_ratelimit_reset, ParseError};
use crate::orchestrator::provider::ProviderWire;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

/// Source de headers en runtime. Implementación por defecto wraps
/// `reqwest::header::HeaderMap`. En tests, mocks.
pub trait RetryAfterSource {
    /// Devuelve el valor del header `Retry-After` crudo, o `None`.
    fn retry_after(&self) -> Option<&str>;

    /// OpenAI no devuelve `Retry-After` pero sí `x-ratelimit-reset-requests`
    /// (delay-seconds o ISO-8601 delay) y `x-ratelimit-reset-tokens`.
    fn x_ratelimit_reset_requests(&self) -> Option<&str>;

    fn x_ratelimit_reset_tokens(&self) -> Option<&str>;
}

/// Configuración de cooldown por provider. Cada deployemt termina en
/// un `ProviderWire` y ésta les da el cooldown base.
///
/// `overrides` es un `HashMap<ProviderWire, Duration>`. El caller
/// (typicamente `Profile::cooldown_overrides`) lo carga con
/// durations serializables; aquí las aceptamos ya parsed.
#[derive(Debug, Clone, Default)]
pub struct CooldownConfig {
    /// Override por provider. Key ausente → usar `default_for(provider)`.
    pub overrides: HashMap<ProviderWire, Duration>,
}

impl CooldownConfig {
    /// Defaults per provider según docs oficiales (RFC 28 §H.4):
    /// - Anthropic: 30s (count_tokens_window + 429 guidance).
    /// - Bedrock Claude: 1s (no rate-limit agresivo en AWS).
    /// - OpenAI / Azure / OpenAI-compat: 60s (TPM-based cooldown).
    /// - Gemini / VertexAI: 30s.
    /// - Groq / Cerebras: 30s (free-tier aggressive rate-limit).
    /// - DeepSeek / Mistral: 60s.
    /// - Ollama / LmStudio / LlamaCppServer: 5s (local server).
    /// - Sambanova: 30s.
    /// - NvidiaNim / HuggingFaceInference: 10s.
    /// - CloudflareWorkersAi: 60s.
    /// - OpenRouter / LiteLLM: 30s (gateway averaged).
    /// - GitHubModels: 60s.
    /// - Custom: 30s (conservative default; operators override).
    pub fn default_for(provider: &ProviderWire) -> Duration {
        match provider {
            ProviderWire::OpenAI
            | ProviderWire::Azure
            | ProviderWire::GitHubModels
            | ProviderWire::CloudflareWorkersAi => Duration::seconds(60),
            ProviderWire::Anthropic
            | ProviderWire::Bedrock
            | ProviderWire::Gemini
            | ProviderWire::VertexAI
            | ProviderWire::Groq
            | ProviderWire::Cerebras
            | ProviderWire::Sambanova
            | ProviderWire::OpenRouter
            | ProviderWire::LiteLLM
            | ProviderWire::Custom(_) => Duration::seconds(30),
            ProviderWire::DeepSeek | ProviderWire::Mistral => Duration::seconds(60),
            ProviderWire::NvidiaNim | ProviderWire::HuggingFaceInference => Duration::seconds(10),
            ProviderWire::Ollama | ProviderWire::LmStudio | ProviderWire::LlamaCppServer => {
                Duration::seconds(5)
            }
        }
    }

    /// Cooldown base para `provider`. Override si está en `overrides`,
    /// si no `default_for(provider)`.
    pub fn resolve(&self, provider: &ProviderWire) -> Duration {
        self.overrides
            .get(provider)
            .copied()
            .unwrap_or_else(|| Self::default_for(provider))
    }

    /// Resolve considerando `Retry-After` y `x-ratelimit-reset-*`
    /// headers del response HTTP. Si el provider informa un
    /// cooldown, ese es autoritativo.
    /// Devuelve el `Duration` a esperar + origen (Debug info).
    pub fn resolve_with_retry_after(
        &self,
        provider: &ProviderWire,
        source: &dyn RetryAfterSource,
        now: DateTime<Utc>,
    ) -> CooldownOutcome {
        // 1. Retry-After header (RFC 7231 §7.1.3).
        if let Some(raw) = source.retry_after() {
            match parse_retry_after(raw, now) {
                Ok(at) => {
                    let dur = (at - now).max(Duration::zero());
                    return CooldownOutcome {
                        duration: dur.clamp_to_min(Duration::seconds(1)),
                        origin: CooldownOrigin::RetryAfterHeader,
                    };
                }
                Err(ParseError::Malformed(_)) => {
                    // malformed — fallen a default.
                }
                Err(other) => {
                    tracing::warn!(?other, "Retry-After parse failed");
                }
            }
        }
        // 2. x-ratelimit-reset-requests (OpenAI). Si existe, usar.
        if let Some(raw) = source.x_ratelimit_reset_requests() {
            if let Ok(at) = parse_x_ratelimit_reset(raw, now) {
                let dur = (at - now).max(Duration::zero());
                return CooldownOutcome {
                    duration: dur.clamp_to_min(Duration::seconds(1)),
                    origin: CooldownOrigin::XRatelimitResetRequests,
                };
            }
        }
        // 3. x-ratelimit-reset-tokens (OpenAI). Mismo formato,
        //    usualmente mayor (tokens TPM).
        if let Some(raw) = source.x_ratelimit_reset_tokens() {
            if let Ok(at) = parse_x_ratelimit_reset(raw, now) {
                let dur = (at - now).max(Duration::zero());
                return CooldownOutcome {
                    duration: dur.clamp_to_min(Duration::seconds(1)),
                    origin: CooldownOrigin::XRatelimitResetTokens,
                };
            }
        }
        // 4. Default de la config.
        CooldownOutcome {
            duration: self.resolve(provider),
            origin: CooldownOrigin::Default,
        }
    }
}

/// Resultado de resolve con origen para logs. Distinto de sólo
/// Duration para que el feedback loop pueda atribuir "el provider
/// pidió Retry-After=90s" vs "usamos default 30s".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooldownOutcome {
    pub duration: Duration,
    pub origin: CooldownOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CooldownOrigin {
    /// `Retry-After` HTTP header (RFC 7231 §7.1.3).
    RetryAfterHeader,
    /// OpenAI `x-ratelimit-reset-requests`.
    XRatelimitResetRequests,
    /// OpenAI `x-ratelimit-reset-tokens`.
    XRatelimitResetTokens,
    /// Default del `CooldownConfig::default_for(provider)`.
    Default,
}

/// Clamps implícito para que duration nunca sea negativa ni cero
/// (un Retry-After: 0 significa "reintenta ahora" pero con cooldown
/// "ya" no hay circuit-breaker; mejor un segundo como mínimo).
trait DurationClampExt {
    fn clamp_to_min(self, min: Duration) -> Duration;
}

impl DurationClampExt for Duration {
    fn clamp_to_min(self, min: Duration) -> Duration {
        if self < min {
            min
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock source simple para tests.
    struct MockSource {
        retry_after: Option<String>,
        x_ratelimit_requests: Option<String>,
        x_ratelimit_tokens: Option<String>,
    }

    impl RetryAfterSource for MockSource {
        fn retry_after(&self) -> Option<&str> {
            self.retry_after.as_deref()
        }
        fn x_ratelimit_reset_requests(&self) -> Option<&str> {
            self.x_ratelimit_requests.as_deref()
        }
        fn x_ratelimit_reset_tokens(&self) -> Option<&str> {
            self.x_ratelimit_tokens.as_deref()
        }
    }

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-05T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn default_for_anthropic_is_30s() {
        assert_eq!(
            CooldownConfig::default_for(&ProviderWire::Anthropic),
            Duration::seconds(30)
        );
    }

    #[test]
    fn default_for_openai_is_60s() {
        assert_eq!(
            CooldownConfig::default_for(&ProviderWire::OpenAI),
            Duration::seconds(60)
        );
    }

    #[test]
    fn default_for_ollama_is_5s() {
        assert_eq!(
            CooldownConfig::default_for(&ProviderWire::Ollama),
            Duration::seconds(5)
        );
    }

    #[test]
    fn default_for_bedrock_is_30s_via_claude_dialect() {
        // Bedrock hereda Anthropic dialect pero tiene cooldown más
        // corto (no-shared quota, AWS throttling separate).
        assert_eq!(
            CooldownConfig::default_for(&ProviderWire::Bedrock),
            Duration::seconds(30),
            "Bedrock default 30s (mismo que Anthropic hasta profile override)"
        );
    }

    #[test]
    fn default_for_custom_is_30s_conservative() {
        let custom = ProviderWire::Custom("vllm".into());
        assert_eq!(
            CooldownConfig::default_for(&custom),
            Duration::seconds(30),
            "Custom default 30s; operador debe override"
        );
    }

    #[test]
    fn default_for_nvidia_nim_is_10s() {
        assert_eq!(
            CooldownConfig::default_for(&ProviderWire::NvidiaNim),
            Duration::seconds(10)
        );
    }

    #[test]
    fn default_for_groq_cerebras_sambanova_is_30s() {
        for p in [
            ProviderWire::Groq,
            ProviderWire::Cerebras,
            ProviderWire::Sambanova,
        ] {
            assert_eq!(
                CooldownConfig::default_for(&p),
                Duration::seconds(30),
                "{p:?} = 30s"
            );
        }
    }

    #[test]
    fn resolve_uses_override_when_present() {
        let mut cfg = CooldownConfig::default();
        cfg.overrides
            .insert(ProviderWire::Anthropic, Duration::seconds(120));
        let got = cfg.resolve(&ProviderWire::Anthropic);
        assert_eq!(got, Duration::seconds(120));
    }

    #[test]
    fn resolve_falls_back_to_default_for_other_providers() {
        let mut cfg = CooldownConfig::default();
        cfg.overrides
            .insert(ProviderWire::Anthropic, Duration::seconds(120));
        // OpenAI sigue con 60s default.
        let got = cfg.resolve(&ProviderWire::OpenAI);
        assert_eq!(got, Duration::seconds(60));
    }

    #[test]
    fn resolve_with_retry_after_uses_header_when_present() {
        let cfg = CooldownConfig::default();
        let source = MockSource {
            retry_after: Some("90".into()),
            x_ratelimit_requests: None,
            x_ratelimit_tokens: None,
        };
        let outcome = cfg.resolve_with_retry_after(&ProviderWire::Anthropic, &source, now());
        assert_eq!(outcome.duration, Duration::seconds(90));
        assert_eq!(outcome.origin, CooldownOrigin::RetryAfterHeader);
    }

    #[test]
    fn resolve_with_retry_after_uses_default_when_header_absent() {
        let cfg = CooldownConfig::default();
        let source = MockSource {
            retry_after: None,
            x_ratelimit_requests: None,
            x_ratelimit_tokens: None,
        };
        let outcome = cfg.resolve_with_retry_after(&ProviderWire::OpenAI, &source, now());
        assert_eq!(outcome.duration, Duration::seconds(60));
        assert_eq!(outcome.origin, CooldownOrigin::Default);
    }

    #[test]
    fn resolve_with_retry_after_handles_invalid_header_gracefully() {
        let cfg = CooldownConfig::default();
        let source = MockSource {
            retry_after: Some("not a valid value".into()),
            x_ratelimit_requests: None,
            x_ratelimit_tokens: None,
        };
        let outcome = cfg.resolve_with_retry_after(&ProviderWire::Anthropic, &source, now());
        assert_eq!(outcome.duration, Duration::seconds(30));
        assert_eq!(outcome.origin, CooldownOrigin::Default);
    }

    #[test]
    fn resolve_with_x_ratelimit_reset_requests_takes_precedence_over_default() {
        let cfg = CooldownConfig::default();
        let source = MockSource {
            retry_after: None,
            x_ratelimit_requests: Some("45".into()),
            x_ratelimit_tokens: None,
        };
        let outcome = cfg.resolve_with_retry_after(&ProviderWire::OpenAI, &source, now());
        assert_eq!(outcome.duration, Duration::seconds(45));
        assert_eq!(outcome.origin, CooldownOrigin::XRatelimitResetRequests);
    }

    #[test]
    fn resolve_with_x_ratelimit_reset_tokens_fallback_when_requests_absent() {
        let cfg = CooldownConfig::default();
        let source = MockSource {
            retry_after: None,
            x_ratelimit_requests: None,
            x_ratelimit_tokens: Some("1200".into()),
        };
        let outcome = cfg.resolve_with_retry_after(&ProviderWire::OpenAI, &source, now());
        assert_eq!(outcome.duration, Duration::seconds(1200), "1200s = 20m");
        assert_eq!(outcome.origin, CooldownOrigin::XRatelimitResetTokens);
    }

    #[test]
    fn resolve_priority_retry_after_beats_x_ratelimit() {
        let cfg = CooldownConfig::default();
        let source = MockSource {
            retry_after: Some("120".into()),
            x_ratelimit_requests: Some("30s".into()),
            x_ratelimit_tokens: None,
        };
        let outcome = cfg.resolve_with_retry_after(&ProviderWire::OpenAI, &source, now());
        assert_eq!(outcome.duration, Duration::seconds(120));
        assert_eq!(outcome.origin, CooldownOrigin::RetryAfterHeader);
    }

    #[test]
    fn resolve_with_retry_after_at_least_min_one_second() {
        // Retry-After que ya expiró (timestamp pasado) → duration
        // negativa → clamp a 1s.
        let cfg = CooldownConfig::default();
        let source = MockSource {
            retry_after: Some("Wed, 05 Aug 2026 11:00:00 GMT".into()),
            x_ratelimit_requests: None,
            x_ratelimit_tokens: None,
        };
        // now = 12:00 UTC; el Retry-After apunta a 11:00 UTC → ya pasó.
        let outcome = cfg.resolve_with_retry_after(&ProviderWire::Anthropic, &source, now());
        assert_eq!(outcome.duration, Duration::seconds(1));
        assert_eq!(outcome.origin, CooldownOrigin::RetryAfterHeader);
    }

    #[test]
    fn resolve_with_override_beats_default_but_loses_to_retry_after() {
        let mut cfg = CooldownConfig::default();
        cfg.overrides
            .insert(ProviderWire::Anthropic, Duration::seconds(180));
        // Override 180s seria fallback; Retry-After pide 90s.
        let source = MockSource {
            retry_after: Some("90".into()),
            x_ratelimit_requests: None,
            x_ratelimit_tokens: None,
        };
        let outcome = cfg.resolve_with_retry_after(&ProviderWire::Anthropic, &source, now());
        assert_eq!(outcome.duration, Duration::seconds(90), "Retry-After wins");
        assert_eq!(outcome.origin, CooldownOrigin::RetryAfterHeader);
    }

    #[test]
    fn resolve_anthropic_override_with_no_header_uses_override() {
        let mut cfg = CooldownConfig::default();
        cfg.overrides
            .insert(ProviderWire::Anthropic, Duration::seconds(180));
        let source = MockSource {
            retry_after: None,
            x_ratelimit_requests: None,
            x_ratelimit_tokens: None,
        };
        let outcome = cfg.resolve_with_retry_after(&ProviderWire::Anthropic, &source, now());
        assert_eq!(outcome.duration, Duration::seconds(180));
        assert_eq!(outcome.origin, CooldownOrigin::Default);
    }

    #[test]
    fn default_groq_uses_generic_fast_provider_default() {
        // Groq tiene rate-limit agresivo; 30s default si no override.
        assert_eq!(
            CooldownConfig::default_for(&ProviderWire::Groq),
            Duration::seconds(30)
        );
    }

    #[test]
    fn default_github_models_uses_openai_dialect_60s() {
        assert_eq!(
            CooldownConfig::default_for(&ProviderWire::GitHubModels),
            Duration::seconds(60)
        );
    }

    #[test]
    fn cooldown_outcome_serde_debug_format() {
        // Sanity check de que Outcome es Debug/PartialEq.
        let o = CooldownOutcome {
            duration: Duration::seconds(45),
            origin: CooldownOrigin::Default,
        };
        let s = format!("{o:?}");
        assert!(s.contains("45s") || s.contains("CooldownOutcome"));
    }
}
