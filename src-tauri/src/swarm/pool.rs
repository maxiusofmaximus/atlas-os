// Atlas OS — Pool swarm parallel dispatch (RFC 05 §4/§5/§8, Phase 4 sub-fase 4.2).
// One tokio task per agent behind the orchestrator `BackPressure` semaphore
// (per-provider, RFC 04 §6) plus deterministic file locks (RFC 05 §4) taken
// before any work runs. `SwarmRunner` walks the RFC 05 §2 topology —
// planner, research/architect, parallel executors, reviewer, merger — and
// persists one Execution Supervisor checkpoint (RFC 19 §5) per agent.

use std::collections::HashMap;
use std::sync::Arc;

use uuid::Uuid;

use crate::journal::Journal;
use crate::orchestrator::{BackPressure, ProviderWire};
use crate::supervisor::types::{BudgetTally, MissionCheckpoint, MissionPhase};
use crate::swarm::roles::Role;

#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("mission allows at most {max} agents, got {got} (RFC 05 §8)")]
    TooManyAgents { max: usize, got: usize },
    #[error("agent {agent} wants {got} file locks, max is {max} (RFC 05 §8)")]
    TooManyFiles { agent: Uuid, max: usize, got: usize },
    #[error("file already locked: {file} (holder {holder})")]
    FileLocked { file: String, holder: Uuid },
    #[error("back-pressure acquire failed: {0}")]
    BackPressure(String),
    #[error(transparent)]
    Journal(#[from] anyhow::Error),
    #[error("agent task panicked or was cancelled: {0}")]
    Join(String),
}

#[derive(Debug, Default)]
pub struct PoolConfig {
    pub max_agents_per_mission: usize,
    pub max_files_locked_per_agent: usize,
}

impl PoolConfig {
    pub fn with_defaults() -> Self {
        Self {
            max_agents_per_mission: 10,
            max_files_locked_per_agent: 8,
        }
    }
}

#[derive(Debug, Default)]
pub struct FileLockRegistry {
    max_files_per_agent: usize,
    held: parking_lot::Mutex<HashMap<String, Uuid>>,
}

impl FileLockRegistry {
    pub fn new(max_files_per_agent: usize) -> Self {
        Self {
            max_files_per_agent: max_files_per_agent.max(1),
            held: parking_lot::Mutex::new(HashMap::new()),
        }
    }

    pub fn acquire(&self, agent_id: Uuid, files: &[String]) -> Result<(), PoolError> {
        if files.len() > self.max_files_per_agent {
            return Err(PoolError::TooManyFiles {
                agent: agent_id,
                max: self.max_files_per_agent,
                got: files.len(),
            });
        }
        let mut ordered: Vec<&String> = files.iter().collect();
        ordered.sort();
        ordered.dedup();
        let mut held = self.held.lock();
        for file in &ordered {
            if let Some(holder) = held.get(*file) {
                if *holder != agent_id {
                    return Err(PoolError::FileLocked {
                        file: (*file).clone(),
                        holder: *holder,
                    });
                }
            }
        }
        for file in ordered {
            held.insert(file.clone(), agent_id);
        }
        Ok(())
    }

    pub fn release(&self, agent_id: Uuid, files: &[String]) {
        let mut held = self.held.lock();
        for file in files {
            if held.get(file) == Some(&agent_id) {
                held.remove(file);
            }
        }
    }

    pub fn holder_of(&self, file: &str) -> Option<Uuid> {
        self.held.lock().get(file).copied()
    }

    pub fn locked_count(&self) -> usize {
        self.held.lock().len()
    }
}

#[derive(Clone, Debug)]
pub struct PoolAgentSpec {
    pub agent_id: Uuid,
    pub role: Role,
    pub provider: ProviderWire,
    pub model_id: Option<String>,
    pub files: Vec<String>,
}

impl PoolAgentSpec {
    pub fn new(agent_id: Uuid, role: Role, files: Vec<String>) -> Self {
        Self {
            agent_id,
            role,
            provider: ProviderWire::Custom(role.as_str().to_string()),
            model_id: None,
            files,
        }
    }

    pub fn with_provider(mut self, provider: ProviderWire) -> Self {
        self.provider = provider;
        self
    }

    pub fn with_model(mut self, model_id: &str) -> Self {
        self.model_id = Some(model_id.to_string());
        self
    }
}

#[derive(Clone, Debug)]
pub struct PoolOutcome {
    pub agent_id: Uuid,
    pub role: Role,
    pub checkpoint_id: Uuid,
    pub succeeded: bool,
    pub error: Option<String>,
}

fn checkpoint_for(mission_id: &str) -> MissionCheckpoint {
    MissionCheckpoint {
        checkpoint_id: Uuid::new_v4(),
        mission_id: Uuid::parse_str(mission_id).unwrap_or_else(|_| Uuid::nil()),
        phase: MissionPhase::Executing,
        current_plan_id: None,
        last_validation_report_id: None,
        last_repair_id: None,
        budget_tally: BudgetTally::default(),
        generated_at: chrono::Utc::now().to_rfc3339(),
    }
}

pub struct SwarmRunner {
    mission_id: String,
    journal: Arc<Journal>,
    backpressure: Arc<BackPressure>,
    locks: Arc<FileLockRegistry>,
    config: PoolConfig,
}

impl SwarmRunner {
    pub fn new(
        mission_id: String,
        journal: Arc<Journal>,
        backpressure: Arc<BackPressure>,
        locks: Arc<FileLockRegistry>,
    ) -> Self {
        Self {
            mission_id,
            journal,
            backpressure,
            locks,
            config: PoolConfig::with_defaults(),
        }
    }

    pub fn with_config(mut self, config: PoolConfig) -> Self {
        self.config = config;
        self
    }

    pub fn mission_id(&self) -> &str {
        &self.mission_id
    }

    fn finish_agent(
        &self,
        spec: &PoolAgentSpec,
        succeeded: bool,
        error: Option<String>,
    ) -> PoolOutcome {
        let checkpoint = checkpoint_for(&self.mission_id);
        let outcome = PoolOutcome {
            agent_id: spec.agent_id,
            role: spec.role,
            checkpoint_id: checkpoint.checkpoint_id,
            succeeded,
            error,
        };
        if self.journal.save_checkpoint(&checkpoint).is_ok() {
            let state = if succeeded { "done" } else { "failed" };
            let _ = self.journal.set_swarm_agent_state(spec.agent_id, state);
        }
        outcome
    }

    pub async fn run_one<F, Fut>(
        &self,
        spec: PoolAgentSpec,
        work: F,
    ) -> Result<PoolOutcome, PoolError>
    where
        F: FnOnce(PoolAgentSpec) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<()>>,
    {
        if 1 > self.config.max_agents_per_mission {
            return Err(PoolError::TooManyAgents {
                max: self.config.max_agents_per_mission,
                got: 1,
            });
        }
        self.journal
            .set_swarm_agent_state(spec.agent_id, "working")?;
        if let Err(e) = self.locks.acquire(spec.agent_id, &spec.files) {
            return Ok(self.finish_agent(&spec, false, Some(e.to_string())));
        }
        let permit = self
            .backpressure
            .acquire(&spec.provider)
            .await
            .map_err(|e| PoolError::BackPressure(e.to_string()))?;
        let result = work(spec.clone()).await;
        drop(permit);
        self.locks.release(spec.agent_id, &spec.files);
        match result {
            Ok(()) => Ok(self.finish_agent(&spec, true, None)),
            Err(e) => Ok(self.finish_agent(&spec, false, Some(format!("{e:#}")))),
        }
    }

    pub async fn run_parallel<F, Fut>(
        &self,
        specs: Vec<PoolAgentSpec>,
        work: F,
    ) -> Result<Vec<PoolOutcome>, PoolError>
    where
        F: Fn(PoolAgentSpec) -> Fut + Clone + Send + Sync + 'static,
        Fut: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        if specs.len() > self.config.max_agents_per_mission {
            return Err(PoolError::TooManyAgents {
                max: self.config.max_agents_per_mission,
                got: specs.len(),
            });
        }
        let mut handles = Vec::with_capacity(specs.len());
        for spec in specs {
            let journal = Arc::clone(&self.journal);
            let backpressure = Arc::clone(&self.backpressure);
            let locks = Arc::clone(&self.locks);
            let mission_id = self.mission_id.clone();
            let work = work.clone();
            handles.push(tokio::spawn(async move {
                let runner = SwarmRunner {
                    mission_id,
                    journal,
                    backpressure,
                    locks,
                    config: PoolConfig::with_defaults(),
                };
                runner.run_one(spec, work).await
            }));
        }
        let mut outcomes = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(Ok(outcome)) => outcomes.push(outcome),
                Ok(Err(e)) => {
                    let msg = e.to_string();
                    outcomes.push(PoolOutcome {
                        agent_id: Uuid::nil(),
                        role: Role::Backend,
                        checkpoint_id: Uuid::nil(),
                        succeeded: false,
                        error: Some(msg),
                    });
                }
                Err(e) => {
                    return Err(PoolError::Join(e.to_string()));
                }
            }
        }
        outcomes.sort_by_key(|o| o.agent_id);
        Ok(outcomes)
    }

    pub async fn run_topology<F, Fut>(
        &self,
        specs: Vec<PoolAgentSpec>,
        work: F,
    ) -> Result<Vec<PoolOutcome>, PoolError>
    where
        F: Fn(PoolAgentSpec) -> Fut + Clone + Send + Sync + 'static,
        Fut: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        fn phase_of(role: Role) -> u8 {
            match role {
                Role::Planner => 0,
                Role::Researcher | Role::Architect => 1,
                Role::Backend
                | Role::Frontend
                | Role::Database
                | Role::Security
                | Role::Testing => 2,
                Role::Reviewer => 3,
                Role::Merger => 4,
            }
        }
        let mut by_phase: [Vec<PoolAgentSpec>; 5] = Default::default();
        for spec in specs {
            by_phase[phase_of(spec.role) as usize].push(spec);
        }
        let mut all = Vec::new();
        for mut group in by_phase {
            if group.is_empty() {
                continue;
            }
            group.sort_by_key(|s| Role::ALL.iter().position(|r| *r == s.role).unwrap_or(99));
            if group.len() == 1 {
                let spec = group.into_iter().next().expect("len checked");
                all.push(self.run_one(spec, work.clone()).await?);
            } else {
                all.extend(self.run_parallel(group, work.clone()).await?);
            }
        }
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::BackPressureConfig;

    fn harness() -> (
        tempfile::TempDir,
        Arc<Journal>,
        Arc<BackPressure>,
        Arc<FileLockRegistry>,
    ) {
        let dir = tempfile::TempDir::new().unwrap();
        let journal = Arc::new(Journal::open(dir.path()).unwrap());
        let backpressure = Arc::new(BackPressure::new(BackPressureConfig::default()));
        let locks = Arc::new(FileLockRegistry::new(8));
        (dir, journal, backpressure, locks)
    }

    fn register(journal: &Journal, mission: &str, role: Role) -> Uuid {
        let id = Uuid::new_v4();
        journal
            .register_swarm_agent(id, mission, role, None, None)
            .unwrap();
        id
    }

    #[tokio::test]
    async fn pool_runs_n_stub_agents_in_parallel_with_checkpoint_each() {
        let (_dir, journal, backpressure, locks) = harness();
        let mission = Uuid::new_v4().to_string();
        let runner = SwarmRunner::new(mission.clone(), journal.clone(), backpressure, locks);
        let roles = [
            Role::Backend,
            Role::Frontend,
            Role::Database,
            Role::Security,
            Role::Testing,
        ];
        let mut specs = Vec::new();
        for (i, role) in roles.into_iter().enumerate() {
            let id = register(&journal, &mission, role);
            specs.push(PoolAgentSpec::new(
                id,
                role,
                vec![format!("src/mod_{i}.rs")],
            ));
        }
        let outcomes = runner
            .run_parallel(specs, |_| async { Ok(()) })
            .await
            .unwrap();
        assert_eq!(outcomes.len(), 5);
        assert!(outcomes.iter().all(|o| o.succeeded));
        let mut checkpoints = outcomes.iter().map(|o| o.checkpoint_id).collect::<Vec<_>>();
        checkpoints.sort();
        checkpoints.dedup();
        assert_eq!(checkpoints.len(), 5, "one checkpoint per agent");
        for o in &outcomes {
            let payload = journal.checkpoint_payload(o.checkpoint_id).unwrap();
            assert!(payload.is_some(), "checkpoint persisted for {}", o.agent_id);
        }
        let rows = journal.swarm_agents_for_mission(&mission).unwrap();
        assert_eq!(rows.len(), 5);
        assert!(rows.iter().all(|r| r.state == "done"));
    }

    #[tokio::test]
    async fn file_locks_prevent_two_agents_touching_same_file() {
        let (_dir, journal, backpressure, locks) = harness();
        let mission = Uuid::new_v4().to_string();
        let runner = SwarmRunner::new(mission.clone(), journal.clone(), backpressure, locks);
        let shared = "src/shared.rs".to_string();
        let a = register(&journal, &mission, Role::Backend);
        let b = register(&journal, &mission, Role::Frontend);
        let specs = vec![
            PoolAgentSpec::new(a, Role::Backend, vec![shared.clone()]),
            PoolAgentSpec::new(b, Role::Frontend, vec![shared.clone()]),
        ];
        let outcomes = runner
            .run_parallel(specs, |spec| async move {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                let _ = spec.agent_id;
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(outcomes.len(), 2);
        let ok = outcomes.iter().filter(|o| o.succeeded).count();
        let failed = outcomes.iter().filter(|o| !o.succeeded).count();
        assert_eq!(ok, 1);
        assert_eq!(failed, 1);
        let err = outcomes
            .iter()
            .find(|o| !o.succeeded)
            .and_then(|o| o.error.clone())
            .unwrap();
        assert!(err.contains("already locked"), "unexpected error: {err}");
    }

    #[tokio::test]
    async fn work_failure_marks_agent_failed_with_checkpoint() {
        let (_dir, journal, backpressure, locks) = harness();
        let mission = Uuid::new_v4().to_string();
        let runner = SwarmRunner::new(mission.clone(), journal.clone(), backpressure, locks);
        let id = register(&journal, &mission, Role::Backend);
        let spec = PoolAgentSpec::new(id, Role::Backend, vec!["src/a.rs".to_string()]);
        let outcome = runner
            .run_one(spec, |_| async { anyhow::bail!("boom") })
            .await
            .unwrap();
        assert!(!outcome.succeeded);
        assert!(outcome.error.unwrap().contains("boom"));
        assert!(journal
            .checkpoint_payload(outcome.checkpoint_id)
            .unwrap()
            .is_some());
        let rows = journal.swarm_agents_for_mission(&mission).unwrap();
        assert_eq!(rows[0].state, "failed");
    }

    #[tokio::test]
    async fn pool_rejects_more_agents_than_cap() {
        let (_dir, journal, backpressure, locks) = harness();
        let mission = Uuid::new_v4().to_string();
        let mut runner = SwarmRunner::new(mission.clone(), journal, backpressure, locks);
        runner.config.max_agents_per_mission = 2;
        let specs = (0..3)
            .map(|_| PoolAgentSpec::new(Uuid::new_v4(), Role::Backend, vec![]))
            .collect::<Vec<_>>();
        let err = runner
            .run_parallel(specs, |_| async { Ok(()) })
            .await
            .unwrap_err();
        assert!(matches!(err, PoolError::TooManyAgents { max: 2, got: 3 }));
    }

    #[tokio::test]
    async fn topology_orders_planner_first_merger_last() {
        let (_dir, journal, backpressure, locks) = harness();
        let mission = Uuid::new_v4().to_string();
        let runner = SwarmRunner::new(mission.clone(), journal.clone(), backpressure, locks);
        let order = Arc::new(parking_lot::Mutex::new(Vec::<Role>::new()));
        let mut specs = Vec::new();
        for role in [Role::Merger, Role::Backend, Role::Planner, Role::Reviewer] {
            let id = register(&journal, &mission, role);
            specs.push(PoolAgentSpec::new(
                id,
                role,
                vec![format!("{}.rs", role.as_str())],
            ));
        }
        let seen = Arc::clone(&order);
        let outcomes = runner
            .run_topology(specs, move |spec| {
                let seen = Arc::clone(&seen);
                async move {
                    seen.lock().push(spec.role);
                    Ok(())
                }
            })
            .await
            .unwrap();
        assert_eq!(outcomes.len(), 4);
        let seq = order.lock().clone();
        assert_eq!(seq.first(), Some(&Role::Planner));
        assert_eq!(seq.last(), Some(&Role::Merger));
    }

    #[test]
    fn locks_reject_too_many_files_and_release_frees() {
        let registry = FileLockRegistry::new(1);
        let agent = Uuid::new_v4();
        let err = registry
            .acquire(agent, &["a".to_string(), "b".to_string()])
            .unwrap_err();
        assert!(matches!(err, PoolError::TooManyFiles { .. }));
        registry.acquire(agent, &["a".to_string()]).unwrap();
        assert_eq!(registry.holder_of("a"), Some(agent));
        registry.release(agent, &["a".to_string()]);
        assert_eq!(registry.holder_of("a"), None);
        assert_eq!(registry.locked_count(), 0);
    }
}
