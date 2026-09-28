// Atlas OS — Skill manifest parser + Skill Graph (RFC 06).
//
// Phase 1的实现范围:
//
//   * `SkillManifest` — esquema TOML/YAML completo de RFC 06 §1
//     (engine, priority, domain, language, framework, confidence,
//     estimated_tokens, estimated_time, dependencies, conflicts,
//     compatible_models, vector_embedding, summary, auto_generated,
//     verified, version, author, license, home). Todos los campos
//     nuevos son `#[serde(default)]` así el `skill.toml` Phase-0
//     `prompt-clarify` sigue parseando sin cambios.
//
//   * `SkillGraph` — grafo en memoria con `register` / `lookup_by_id`
//     / `find_candidates` (top-K por `priority`) y la regla de
//     conflictos §8 (el planificador nunca programa dos skills
//     conflictivas juntas). El Bus de embeddings llega en Phase 2 con
//     `fastembed-rs`.
//
//   * `Engine` enum (RFC 06 §2) — el motor al que se asocia
//     automáticamente cada skill. El Kernel enruta al motor correcto
//     usando este campo.
//
// Phase 5 añadirá la compresión / fusión de skills (RFC 06 §5) y el
// hook que el Learning Engine usa para autogenerar skills (§7).

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// RFC 06 §2 — motor al que el Kernel enruta una skill. Mapea 1:1 con
/// la tabla §2 "Asociación automática a un motor".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    Planning,
    Research,
    Coding,
    Validation,
    Security,
    Reasoning,
    Learning,
    /// Sin motor asignado — el usuario declara una skill sin
    /// `engine`. El Kernel deja la skill en el grafo pero nunca la
    /// dispatcha automáticamente.
    #[default]
    Unspecified,
}

impl Engine {
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Planning => "planning",
            Self::Research => "research",
            Self::Coding => "coding",
            Self::Validation => "validation",
            Self::Security => "security",
            Self::Reasoning => "reasoning",
            Self::Learning => "learning",
            Self::Unspecified => "unspecified",
        }
    }

    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        crate::skills::sdk::parse_engine(raw)
    }
}

impl std::str::FromStr for Engine {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        crate::skills::sdk::parse_engine(raw)
    }
}

/// RFC 06 §1 — el manifest completo de una skill. La estructura existe
/// en dos formatos: TOML (`skill.toml` para skills locales) y YAML
/// (RFC 16 §4 `.opencode/rules/*.yaml` generado por el Learning
/// Engine). Phase 1 solo lee TOML; Phase 2 añadirá YAML.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SkillManifest {
    /// Identificador estable. Ej: `react-ui-expert`.
    pub id: String,
    pub version: String,
    pub description: String,
    /// RFC 06 §1 `engine` — routing automático al motor.
    #[serde(default)]
    pub engine: Engine,
    /// RFC 06 §1 `priority` — 0-100. 90 = alta sugerencia, 0 = grisada.
    #[serde(default)]
    pub priority: u32,
    /// RFC 06 §1 `domain` — `frontend`, `backend`, `devops`, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// RFC 06 §1 `language` — `typescript`, `rust`, `go`, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// RFC 06 §1 `framework` — `react`, `vue`, `tauri`, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,
    /// RFC 06 §1 `confidence` — [0, 1]. Phase 2 lo alimentan los
    /// embeddings + history.
    #[serde(default)]
    pub confidence: f32,
    /// RFC 06 §1 `estimated_tokens`. `0` cuando no se conoce.
    #[serde(default)]
    pub estimated_tokens: u32,
    /// RFC 06 §1 `estimated_time` en segundos.
    #[serde(default)]
    pub estimated_time: u32,
    /// RFC 06 §1 `dependencies[]` — skill ids que deben activarse
    /// también.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
    /// RFC 06 §1 `compatible_models[]` — ids de modelos que saben
    /// ejecutar la skill.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compatible_models: Vec<String>,
    /// RFC 06 §1 `vector_embedding`. Phase 1 lo recibimos vacío; el
    /// Learning Engine (RFC 16 §3) lo rellenará con `fastembed-rs`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vector_embedding: Vec<f32>,
    /// RFC 06 §1 `summary` — una línea para la UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// RFC 06 §8 `conflicts[]` — ids de skills que no pueden
    /// coejecutarse sucesivamente.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
    /// RFC 06 §7 `auto_generated` — `true` para skills creadas por
    /// el Learning Engine.
    #[serde(default)]
    pub auto_generated: bool,
    /// RFC 06 §1 `verified` — `true` tras revisión humana / X
    /// ejecuciones correctas.
    #[serde(default)]
    pub verified: bool,
    /// RFC 06 §1 capbed de Phase-0 (sandbox). Mantenido por retrocompat.
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub requires_sandbox: bool,
    /// RFC 06 §1 `author`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// RFC 06 §1 `license`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// RFC 06 §1 `home` — ruta local de la skill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<String>,
}

impl SkillManifest {
    /// RFC 06 §1 — derivación del `version` arquitectónico (public-api
    /// semver). Phase 1 solo usa el campo literal.
    pub fn semver(&self) -> &str {
        &self.version
    }

    /// RFC 06 §7 — una skill auto-generada arranca con `priority=50`
    /// y `verified=false` (RFC 16 preserve hasta X ejecuciones).
    pub fn is_auto_pending_verification(&self) -> bool {
        self.auto_generated && !self.verified
    }
}

/// Cargar una skill desde su directorio. Busca `skill.toml` (Phase 0
/// convention) y devuelve el manifest parseado.
pub fn load_skill(dir: &Path) -> anyhow::Result<SkillManifest> {
    let manifest_path = dir.join("skill.toml");
    let contents = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading skill manifest {}", manifest_path.display()))?;
    let manifest: SkillManifest = toml::from_str(&contents).context("parsing skill.toml")?;
    Ok(manifest)
}

/// RFC 06 §1 — grafo de skills en memoria. Phase 1 mantiene el grafo
/// como un `HashMap<id, SkillManifest>` + un índice de conflictos
/// precalculado. Phase 2 añadirá un índice de embeddings (sqlite-vec)
/// para la búsqueda semántica del pipeline §3.
#[derive(Clone, Debug, Default)]
pub struct SkillGraph {
    skills: HashMap<String, SkillManifest>,
    /// `conflicts_index[A]` = conjunto de ids que entran en conflicto
    /// con A. Bidireccional, precargado en `register`.
    conflicts_index: HashMap<String, HashSet<String>>,
}

impl SkillGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// RFC 06 §1 — registra una skill en el grafo. Si la skill ya
    /// existe (mismo id), la nueva versión reemplaza la anterior
    /// (Phase 2 debería bumpar `version` y auditar el diff). Los
    /// conflictos se registran en el índice bidireccional.
    pub fn register(&mut self, skill: SkillManifest) {
        // Index bidirectional conflicts.
        for other in &skill.conflicts {
            self.conflicts_index
                .entry(skill.id.clone())
                .or_default()
                .insert(other.clone());
            self.conflicts_index
                .entry(other.clone())
                .or_default()
                .insert(skill.id.clone());
        }
        self.skills.insert(skill.id.clone(), skill);
    }

    pub fn lookup_by_id(&self, id: &str) -> Option<&SkillManifest> {
        self.skills.get(id)
    }

    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// RFC 06 §8 — devuelve `true` si `a` y `b` entran en conflicto
    /// (sea cual sea el sentido). El Planner consulta este método
    /// antes de coschedule dos skills.
    pub fn are_in_conflict(&self, a: &str, b: &str) -> bool {
        if a == b {
            return false;
        }
        self.conflicts_index
            .get(a)
            .map(|set| set.contains(b))
            .unwrap_or(false)
    }

    /// RFC 06 §3 pipeline — busca los `top_k` candidatos por
    /// `priority` + filtro opcional por `engine`. Phase 1 ordena solo
    /// por `priority` descendente; Phase 2 combinará embeddings +
    /// relevancia (§3). Devuelve referencias a las skills ordenadas.
    pub fn find_candidates(
        &self,
        engine_filter: Option<Engine>,
        top_k: usize,
    ) -> Vec<&SkillManifest> {
        let mut candidates: Vec<&SkillManifest> = self
            .skills
            .values()
            .filter(|s| match engine_filter {
                Some(e) => s.engine == e,
                None => true,
            })
            .collect();
        // Estable: ordena por priority descending, id ascendente
        // (quiebres determinísticos).
        candidates.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.id.cmp(&b.id)));
        candidates.truncate(top_k);
        candidates
    }

    /// RFC 06 §3 — variante owned que devuelve `SkillManifest` por
    /// valor. Útil para el Planner que necesita mover las candidatos
    /// al `Plan.skills_used`.
    pub fn find_candidates_owned(
        &self,
        engine_filter: Option<Engine>,
        top_k: usize,
    ) -> Vec<SkillManifest> {
        self.find_candidates(engine_filter, top_k)
            .into_iter()
            .cloned()
            .collect()
    }

    /// RFC 06 §8 — dados los `candidate_ids`, devuelve un subconjunto
    /// compatible (sin conflictos mutuos) escrito en orden. Greedy:
    /// añade el primer candidato y descarta todos sus conflictos;
    /// repite hasta vaciar la lista. Esto es lo que el Planner usa
    /// para construir un conjunto coschedulable antes de persisted.
    pub fn conflicts_free_subset(&self, candidate_ids: &[String]) -> Vec<String> {
        let mut remaining: Vec<String> = candidate_ids.to_vec();
        let mut chosen: Vec<String> = vec![];
        while let Some(next) = remaining.first().cloned() {
            // Si `next` conflictúa con algo ya elegido, saltar.
            if chosen.iter().any(|c| self.are_in_conflict(c, &next)) {
                remaining.remove(0);
                continue;
            }
            // Eliminar a todos los que conflictúan con `next` (incluido next mismo).
            let conflicts_with_next: HashSet<String> =
                self.conflicts_index.get(&next).cloned().unwrap_or_default();
            remaining.retain(|id| id != &next && !conflicts_with_next.contains(id));
            chosen.push(next);
        }
        chosen
    }

    /// RFC 06 §6 — escanea `skills_root` recursivamente buscando
    /// `skill.toml`. Cada directorio que contiene un manifest se
    /// registra. Errores parciales se registran vía `tracing::warn!`
    /// y NO abortan el escaneo entero (clave para tolerancia de fans
    /// de skills rotas como las del harness de muestra).
    pub fn scan_directory(&mut self, skills_root: &Path) -> anyhow::Result<usize> {
        if !skills_root.is_dir() {
            return Ok(0);
        }
        let mut loaded = 0usize;
        for entry in std::fs::read_dir(skills_root)
            .with_context(|| format!("scanning skills dir {}", skills_root.display()))?
        {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(error = %e, "skip unreadable dir entry");
                    continue;
                }
            };
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            match load_skill(&path) {
                Ok(mut manifest) => {
                    if manifest.home.is_none() {
                        manifest.home = Some(path.to_string_lossy().into());
                    }
                    tracing::debug!(skill = %manifest.id, "loaded skill");
                    self.register(manifest);
                    loaded += 1;
                }
                Err(e) => {
                    tracing::warn!(error = %e, dir = %path.display(), "skip malformed skill");
                }
            }
        }
        Ok(loaded)
    }

    /// Iterador sobre todas las skills (sin orden garantizado).
    pub fn iter(&self) -> impl Iterator<Item = &SkillManifest> {
        self.skills.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(id: &str, priority: u32, engine: Engine) -> SkillManifest {
        SkillManifest {
            id: id.into(),
            version: "1.0.0".into(),
            description: format!("{id} skill"),
            engine,
            priority,
            ..Default::default()
        }
    }

    #[test]
    fn register_and_lookup() {
        let mut g = SkillGraph::new();
        g.register(skill("a", 80, Engine::Coding));
        assert_eq!(g.len(), 1);
        assert!(g.lookup_by_id("a").is_some());
        assert!(g.lookup_by_id("missing").is_none());
    }

    #[test]
    fn find_candidates_orders_by_priority_desc() {
        let mut g = SkillGraph::new();
        g.register(skill("low", 10, Engine::Coding));
        g.register(skill("mid", 50, Engine::Coding));
        g.register(skill("high", 90, Engine::Coding));
        let out = g.find_candidates(Some(Engine::Coding), 10);
        let ids: Vec<&str> = out.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["high", "mid", "low"]);
    }

    #[test]
    fn find_candidates_filters_by_engine() {
        let mut g = SkillGraph::new();
        g.register(skill("a", 90, Engine::Coding));
        g.register(skill("b", 50, Engine::Validation));
        g.register(skill("c", 80, Engine::Coding));
        let coding = g.find_candidates(Some(Engine::Coding), 10);
        assert_eq!(coding.len(), 2);
        assert_eq!(coding[0].id, "a");
        assert_eq!(coding[1].id, "c");
        let all = g.find_candidates(None, 10);
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn find_candidates_truncates_top_k() {
        let mut g = SkillGraph::new();
        for i in 0..10 {
            g.register(skill(&format!("s{i}"), i * 10, Engine::Coding));
        }
        let top3 = g.find_candidates(Some(Engine::Coding), 3);
        assert_eq!(top3.len(), 3);
        assert_eq!(top3[0].id, "s9");
        assert_eq!(top3[1].id, "s8");
    }

    #[test]
    fn conflicts_are_bidirectional() {
        let mut g = SkillGraph::new();
        let mut a = skill("react", 90, Engine::Coding);
        a.conflicts = vec!["vue".into()];
        let mut b = skill("vue", 90, Engine::Coding);
        b.conflicts = vec!["react".into()];
        g.register(a);
        g.register(b);
        assert!(g.are_in_conflict("react", "vue"));
        assert!(g.are_in_conflict("vue", "react"));
        assert!(!g.are_in_conflict("react", "react"));
        assert!(!g.are_in_conflict("react", "ghost"));
    }

    #[test]
    fn conflicts_free_subset_drops_conflicting() {
        let mut g = SkillGraph::new();
        let mut react = skill("react", 90, Engine::Coding);
        react.conflicts = vec!["vue".into()];
        let mut vue = skill("vue", 90, Engine::Coding);
        vue.conflicts = vec!["react".into()];
        g.register(react);
        g.register(vue);
        g.register(skill("svelte", 80, Engine::Coding));

        let chosen = g.conflicts_free_subset(&["vue".into(), "react".into(), "svelte".into()]);
        // `vue` wins (first), `react` dropped (conflicts with vue),
        // svelte kept.
        assert!(chosen.contains(&"vue".to_string()));
        assert!(!chosen.contains(&"react".to_string()));
        assert!(chosen.contains(&"svelte".to_string()));
    }

    #[test]
    fn phase0_skill_toml_still_parses() {
        // The sample skill in the repo only sets id/version/description/
        // capabilities/requires_sandbox/verified — all the new RFC 06
        // §1 fields must default gracefully.
        let toml_src = r#"
            id = "prompt-clarify"
            version = "0.1.0"
            description = "Annotates a user prompt"
            capabilities = ["prompt_understanding"]
            requires_sandbox = false
            verified = true
        "#;
        let manifest: SkillManifest = toml::from_str(toml_src).unwrap();
        assert_eq!(manifest.id, "prompt-clarify");
        assert_eq!(manifest.engine, Engine::Unspecified);
        assert_eq!(manifest.priority, 0);
        assert!(manifest.conflicts.is_empty());
        assert!(manifest.vector_embedding.is_empty());
        assert!(!manifest.auto_generated);
        assert!(manifest.verified);
    }

    #[test]
    fn full_rfc06_manifest_parses() {
        let toml_src = r#"
            id = "react-ui-expert"
            version = "1.4.2"
            description = "Generación de componentes React accesibles"
            engine = "coding"
            priority = 90
            domain = "frontend"
            language = "typescript"
            framework = "react"
            confidence = 0.88
            estimated_tokens = 4200
            estimated_time = 18
            dependencies = ["tailwind-expert", "accessibility-audit"]
            compatible_models = ["claude-sonnet-4.5", "local-qwen2.5-14b-instruct"]
            summary = "Componentes React accesibles"
            conflicts = ["vue-ui-expert"]
            auto_generated = false
            verified = true
            author = "atlas-os"
            license = "MIT"
            home = "./skills/react-ui-expert/"
        "#;
        let manifest: SkillManifest = toml::from_str(toml_src).unwrap();
        assert_eq!(manifest.id, "react-ui-expert");
        assert_eq!(manifest.engine, Engine::Coding);
        assert_eq!(manifest.priority, 90);
        assert!((manifest.confidence - 0.88).abs() < 1e-6);
        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(manifest.conflicts, vec!["vue-ui-expert"]);
    }

    #[test]
    fn is_auto_pending_verification() {
        let mut s = skill("auto", 50, Engine::Learning);
        assert!(!s.is_auto_pending_verification());
        s.auto_generated = true;
        assert!(s.is_auto_pending_verification());
        s.verified = true;
        assert!(!s.is_auto_pending_verification());
    }
}
