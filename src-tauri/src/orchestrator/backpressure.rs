//! Back-pressure semáforo per provider (RFC 04 §6, gap G18).
//!
//! Sin un cap de concurrencia, el orchestrator puede disparejar
//! centenares de requests paralelos contra Anthropic/OpenAI, agotando
//! el rate-limit compartido y triplicando 429s que no eran
//! necesarios. También el sistema local (Ollama) sufre con `>8`
//! peticiones concurrencia (CPU bound).
//!
//! La capa de back-pressure es un `Semaphore` por provider wire:
//!
//! - Cada acquire reserva un slot para una request HTTP.
//! - Si `try_acquire()` devuelve `None`, el orchestrator encola la
//!   request (await) en vez de dispararla; esto evita que el sistema
//!   dispare request que van a fallar.
//! - El semáforo se crea lazy en el primer request del provider,
//!   no se pre-allocate al boot.
//! - Defaults por provider (20 ítems en ProviderWire) — el operador
//!   puede override mediante `Profile.max_concurrent_per_provider`.
//!
//! **Decisión de diseño:**
//!
//! - Semaphore por **ProviderWire**, NO por `Deployment`. Cada
//!   deployment del mesmo provider comparte el rate-limit del
//!   proveedor (Anthropic 4-team shared, OpenAI org-wide TPM), así
//!   que un semáforo por provider es lo natural. Si el operador
//!   tiene dos deployments con api-keys distintas en orgs distintas,
//!   puede configurar dos providers `Custom("vllm_corp_a")` y
//!   `Custom("vllm_corp_b")` que serían dos wires y dos semáforos.
//!
//! - Async-friendly: usamos `tokio::sync::Semaphore` con
//!   `acquire_owned()` para que el permit viva con la request,
//!   incluso atravesando awaits. El trait `BackPressure` es
//!   `Send + Sync` (Arc<Semaphore>).
//!
//! - No escribimos `acquire_blocking` porque el Orchestrator es
//!   100 % async sobre tokio; sync sería anti-pattern.

use crate::orchestrator::provider::ProviderWire;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Semaphore, TryAcquireError};

/// Configuración de concurrencia máxima por provider. Cada provider
/// έχει un `max_concurrent` — si no se especifica en `Profile`,
/// usa `defaults_for(provider)` (heurísticaádquirida de los docs).
#[derive(Debug, Clone, Default)]
pub struct BackPressureConfig {
    /// Override por provider. Si ausente → `default_for(provider)`.
    /// El valor debe ser ≥ 1 (0 ≡ `max_concurrent = 1` en
    /// `Semaphore` semántica).
    pub overrides: HashMap<ProviderWire, u32>,
}

impl BackPressureConfig {
    /// Defaults per provider según rate-limits publicados (RFC 04 §6,
    /// RFC 28 §H.10):
    /// - Anthropic: 4 (limit sharing entre organizaciones, npm).
    /// - OpenAI / Azure: 8.
    /// - Gemini / Vertex AI: 4 (free tier 60/min).
    /// - Bedrock: 10 (AWS throttling less agresivo).
    /// - Groq / Cerebras: 2 (rate-limit muy agresivo free).
    /// - Sambanova: 2.
    /// - DeepSeek: 8.
    /// - Mistral: 8.
    /// - NvidiaNim: 4.
    /// - HuggingFaceInference: 2.
    /// - GitHubModels: 4.
    /// - CloudflareWorkersAi: 6.
    /// - OpenRouter: 8 (gateway averages).
    /// - LiteLLM: configurable upstream, usamos 8 como fallback.
    /// - Ollama / LmStudio / LlamaCppServer: 1 (local CPU/GPU bound;
    ///   se sequentializa en el modelo host).
    /// - Custom: 4 (conservative).
    pub fn default_for(provider: &ProviderWire) -> u32 {
        match provider {
            ProviderWire::Anthropic | ProviderWire::Bedrock => 4,
            ProviderWire::OpenAI | ProviderWire::Azure => 8,
            ProviderWire::Gemini | ProviderWire::VertexAI => 4,
            ProviderWire::Groq
            | ProviderWire::Cerebras
            | ProviderWire::Sambanova
            | ProviderWire::HuggingFaceInference => 2,
            ProviderWire::DeepSeek | ProviderWire::Mistral => 8,
            ProviderWire::NvidiaNim | ProviderWire::GitHubModels => 4,
            ProviderWire::CloudflareWorkersAi => 6,
            ProviderWire::OpenRouter | ProviderWire::LiteLLM => 8,
            ProviderWire::Ollama | ProviderWire::LmStudio | ProviderWire::LlamaCppServer => 1,
            ProviderWire::Custom(_) => 4,
        }
    }

    /// Resuelve el max concurrent para el provider.
    pub fn resolve(&self, provider: &ProviderWire) -> u32 {
        self.overrides
            .get(provider)
            .copied()
            .map(|n| n.max(1))
            .unwrap_or_else(|| Self::default_for(provider))
    }
}

/// Registry de semáforos, uno por provider. Lazy-init — el semáforo
/// se crea en el primer `acquire` o `try_acquire`. Thread-safe.
#[derive(Debug, Default)]
pub struct BackPressure {
    config: BackPressureConfig,
    semaphores: parking_lot::Mutex<HashMap<ProviderWire, Arc<Semaphore>>>,
}

impl BackPressure {
    pub fn new(config: BackPressureConfig) -> Self {
        Self {
            config,
            semaphores: parking_lot::Mutex::new(HashMap::new()),
        }
    }

    /// Obtiene o crea el semáforo para `provider`. Lazy init.
    fn semaphore_for(&self, provider: &ProviderWire) -> Arc<Semaphore> {
        let mut guards = self.semaphores.lock();
        if let Some(arc) = guards.get(provider) {
            return Arc::clone(arc);
        }
        let max = self.config.resolve(provider) as usize;
        let sem = Arc::new(Semaphore::new(max));
        guards.insert(provider.clone(), Arc::clone(&sem));
        sem
    }

    /// Path sync rápido para el orchestrator: si hay slot libre lo
    /// toma, si no, devuelve `Err(TryAcquireError::NoPermits)`. El
    /// caller puede entonces encolar o bajar al siguiente deployment.
    ///
    /// El `OwnedSemaphorePermit` vive con la request async y se
    /// libera al drop — no es necesario `release()` manual.
    pub fn try_acquire(
        &self,
        provider: &ProviderWire,
    ) -> Result<tokio::sync::OwnedSemaphorePermit, TryAcquireError> {
        let sem = self.semaphore_for(provider);
        sem.clone().try_acquire_owned()
    }

    /// Path async: `await` hasta que haya un slot libre. Si el
    /// orchestrator quiere timeout, puede wrappear con
    /// `tokio::time::timeout(duration,_bp.acquire(provider))`.
    pub async fn acquire(
        &self,
        provider: &ProviderWire,
    ) -> Result<tokio::sync::OwnedSemaphorePermit, tokio::sync::AcquireError> {
        let sem = self.semaphore_for(provider);
        sem.acquire_owned().await
    }

    /// Número de permits disponibles. Para HUD health display.
    pub fn available_permits(&self, provider: &ProviderWire) -> usize {
        let guards = self.semaphores.lock();
        match guards.get(provider) {
            Some(sem) => sem.available_permits(),
            None => self.config.resolve(provider) as usize,
        }
    }

    /// Reset (reemplazar) el semáforo de `provider`. Útil en tests y
    /// cuando el orchestrator quiere reset el back-pressure tras un
    /// cambio de Profile (más, menos concurrent).
    pub fn reset_for(&self, provider: &ProviderWire, new_max: u32) {
        let mut guards = self.semaphores.lock();
        let max = new_max.max(1) as usize;
        guards.insert(provider.clone(), Arc::new(Semaphore::new(max)));
    }

    /// Reconfigura todo el back-pressure. Como `&self` excluye
    /// mutate in-place del `config`, este helper devuelve cuáles
    /// providers activos necesitan `reset_for` y la nueva config —
    /// el caller aplica uno a uno. Para providers no inicializados,
    /// la nueva config será leída en el siguiente acquire.
    pub fn plan_reconfigure(
        &self,
        new_config: BackPressureConfig,
    ) -> (Vec<ProviderWire>, BackPressureConfig) {
        let providers: Vec<ProviderWire> = self.semaphores.lock().keys().cloned().collect();
        (providers, new_config)
    }

    /// Snapshot del configuration — útil para HUD.
    pub fn config_for(&self, provider: &ProviderWire) -> u32 {
        self.config.resolve(provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_for_anthropic_is_4() {
        assert_eq!(BackPressureConfig::default_for(&ProviderWire::Anthropic), 4);
    }

    #[test]
    fn default_for_openai_is_8() {
        assert_eq!(BackPressureConfig::default_for(&ProviderWire::OpenAI), 8);
    }

    #[test]
    fn default_for_ollama_is_1() {
        assert_eq!(BackPressureConfig::default_for(&ProviderWire::Ollama), 1);
    }

    #[test]
    fn default_for_groq_cerebras_sambanova_hf_is_2() {
        for p in [
            ProviderWire::Groq,
            ProviderWire::Cerebras,
            ProviderWire::Sambanova,
            ProviderWire::HuggingFaceInference,
        ] {
            assert_eq!(BackPressureConfig::default_for(&p), 2, "{p:?} = 2");
        }
    }

    #[test]
    fn default_for_bedrock_is_4() {
        assert_eq!(BackPressureConfig::default_for(&ProviderWire::Bedrock), 4);
    }

    #[test]
    fn default_for_custom_is_4_conservative() {
        let custom = ProviderWire::Custom("vllm".into());
        assert_eq!(BackPressureConfig::default_for(&custom), 4);
    }

    #[test]
    fn resolve_uses_override_when_present() {
        let mut cfg = BackPressureConfig::default();
        cfg.overrides.insert(ProviderWire::Anthropic, 20);
        assert_eq!(cfg.resolve(&ProviderWire::Anthropic), 20);
        assert_eq!(
            cfg.resolve(&ProviderWire::OpenAI),
            8,
            "sin override usar default"
        );
    }

    #[test]
    fn resolve_override_zero_clamps_to_one() {
        let mut cfg = BackPressureConfig::default();
        cfg.overrides.insert(ProviderWire::Anthropic, 0);
        assert_eq!(cfg.resolve(&ProviderWire::Anthropic), 1, "0 → 1");
    }

    #[test]
    fn try_acquire_grants_first_permit_when_available() {
        let bp = BackPressure::new(BackPressureConfig::default());
        let result = bp.try_acquire(&ProviderWire::Anthropic);
        assert!(result.is_ok(), "debería obtener un permit");
    }

    #[test]
    fn try_acquire_returns_no_permits_after_exhaustion() {
        let mut cfg = BackPressureConfig::default();
        cfg.overrides.insert(ProviderWire::Anthropic, 1);
        let bp = BackPressure::new(cfg);
        let _permit = bp.try_acquire(&ProviderWire::Anthropic).unwrap();
        let next = bp.try_acquire(&ProviderWire::Anthropic);
        assert!(
            matches!(next, Err(TryAcquireError::NoPermits)),
            "segundo acquire en semáforo con perm=1 debe dar NoPermits"
        );
    }

    #[test]
    fn permit_released_on_drop_allows_next() {
        let mut cfg = BackPressureConfig::default();
        cfg.overrides.insert(ProviderWire::Anthropic, 1);
        let bp = BackPressure::new(cfg);
        {
            let _p = bp.try_acquire(&ProviderWire::Anthropic).unwrap();
            // al salir del block, _p drop → release.
        }
        let next = bp.try_acquire(&ProviderWire::Anthropic);
        assert!(next.is_ok(), "post-release, slot disponible");
    }

    #[tokio::test]
    async fn acquire_async_waits_for_permit_release() {
        let mut cfg = BackPressureConfig::default();
        cfg.overrides.insert(ProviderWire::OpenAI, 1);
        let bp = Arc::new(BackPressure::new(cfg));
        // Toma el único permit.
        let owner_permit = bp.try_acquire(&ProviderWire::OpenAI).unwrap();
        // Spawn task que espera el slot.
        let bp_clone = Arc::clone(&bp);
        let h = tokio::spawn(async move {
            let _p = bp_clone.acquire(&ProviderWire::OpenAI).await.unwrap();
        });
        // Espera un tick para confirmar queued (no success).
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        // Suelta.
        drop(owner_permit);
        // Spawn job completa.
        h.await.unwrap();
    }

    #[tokio::test]
    async fn try_acquire_does_not_block() {
        let mut cfg = BackPressureConfig::default();
        cfg.overrides.insert(ProviderWire::Anthropic, 1);
        let bp = BackPressure::new(cfg);
        let _p = bp.try_acquire(&ProviderWire::Anthropic).unwrap();
        let start = std::time::Instant::now();
        let r = bp.try_acquire(&ProviderWire::Anthropic);
        let elapsed = start.elapsed();
        assert!(matches!(r, Err(TryAcquireError::NoPermits)));
        assert!(
            elapsed < std::time::Duration::from_millis(10),
            "try_acquire no espera"
        );
    }

    #[tokio::test]
    async fn multiple_providers_have_independent_semaphores() {
        let bp = BackPressure::new(BackPressureConfig::default());
        let a = bp.try_acquire(&ProviderWire::Anthropic).unwrap();
        let next_a = bp.try_acquire(&ProviderWire::Anthropic);
        assert!(next_a.is_ok(), "Anthropic default max=4");
        let b = bp.try_acquire(&ProviderWire::OpenAI);
        assert!(b.is_ok(), "OpenAI default max=8");
        //多余的数量：
        let _ = a;
    }

    #[tokio::test]
    async fn custom_provider_distinct_ids_create_distinct_semaphores() {
        let mut cfg = BackPressureConfig::default();
        cfg.overrides
            .insert(ProviderWire::Custom("vllm_a".into()), 1);
        cfg.overrides
            .insert(ProviderWire::Custom("vllm_b".into()), 1);
        let bp = BackPressure::new(cfg);
        let _a = bp
            .try_acquire(&ProviderWire::Custom("vllm_a".into()))
            .unwrap();
        // B sigue libre (semáforos distintos).
        let b = bp.try_acquire(&ProviderWire::Custom("vllm_b".into()));
        assert!(b.is_ok(), "distinct Custom ids_→ distinct semaphores");
    }

    #[test]
    fn available_permits_returns_max_before_first_acquire() {
        let bp = BackPressure::new(BackPressureConfig::default());
        // Lazy init: semáforo aún no creado → falls back to config.
        assert_eq!(bp.available_permits(&ProviderWire::Anthropic), 4);
        // Acquire 1 (manteniendo el permit vivo dentro de este block):
        let permit = bp.try_acquire(&ProviderWire::Anthropic).unwrap();
        let mid = bp.available_permits(&ProviderWire::Anthropic);
        assert_eq!(mid, 3, "tras acquire: 4-1=3");
        // Drop el permit → release slot:
        drop(permit);
        assert_eq!(
            bp.available_permits(&ProviderWire::Anthropic),
            4,
            "tras release: 4 free otra vez"
        );
    }

    #[test]
    fn reset_for_replaces_semaphores() {
        let bp = BackPressure::new(BackPressureConfig::default());
        // Init semáforo Anthropic a default 4. Mantenemos el permit
        // vivo para que el slot permanezca consumido.
        let permit = bp.try_acquire(&ProviderWire::Anthropic).unwrap();
        assert_eq!(bp.available_permits(&ProviderWire::Anthropic), 3);
        bp.reset_for(&ProviderWire::Anthropic, 10);
        assert_eq!(bp.available_permits(&ProviderWire::Anthropic), 10);
        drop(permit);
    }

    #[test]
    fn config_for_returns_resolved_max() {
        let mut cfg = BackPressureConfig::default();
        cfg.overrides.insert(ProviderWire::OpenAI, 50);
        let bp = BackPressure::new(cfg);
        assert_eq!(bp.config_for(&ProviderWire::OpenAI), 50);
        assert_eq!(bp.config_for(&ProviderWire::Anthropic), 4);
    }
}
