# PROGRESS — Ronda 2 (Terminal-Bench 2 completo, 89 tareas)

Modelo: `longcat-2.5-preview-free` · agente: `harbor_atlas.atlas_agent:AtlasAgent` (`ATLAS_AGENT_MODE=agent`) · `--n-concurrent 2` · 1 job · guard `MAX_REQ=4000`.

Base URL = env-file (`https://opencode.ai/zen/go/v1`); clave inyectada por `--ae` y redactada en `jobs/<job>/**/config.json`.

## Progreso
- 12:35 `calib-longcat-2.5-preview-free-20261008-123322` completed=0/89 running=2 pass1.0=0 pass_rate=0.000 errored=0 rewards={} exceptions={}
- 12:37 `calib-longcat-2.5-preview-free-20261008-123322` completed=0/89 running=2 pass1.0=0 pass_rate=0.000 errored=0 rewards={} exceptions={}
- 13:08 `calib-longcat-2.5-preview-free-20261008-123322` completed=1/89 running=2 pass1.0=0 pass_rate=0.000 errored=1 rewards={'0.0': 1} exceptions={'AgentTimeoutError': 1}
- 13:39 `calib-longcat-2.5-preview-free-20261008-123322` completed=5/89 running=2 pass1.0=1 pass_rate=0.200 errored=2 rewards={'0.0': 4, '1.0': 1} exceptions={'AgentTimeoutError': 2}
- 14:11 `calib-longcat-2.5-preview-free-20261008-123322` completed=6/89 running=2 pass1.0=1 pass_rate=0.167 errored=3 rewards={'0.0': 5, '1.0': 1} exceptions={'AgentTimeoutError': 3}
- 14:42 `calib-longcat-2.5-preview-free-20261008-123322` completed=9/89 running=2 pass1.0=1 pass_rate=0.111 errored=3 rewards={'0.0': 8, '1.0': 1} exceptions={'AgentTimeoutError': 3}
- 15:13 `calib-longcat-2.5-preview-free-20261008-123322` completed=11/89 running=2 pass1.0=1 pass_rate=0.091 errored=5 rewards={'0.0': 10, '1.0': 1} exceptions={'AgentTimeoutError': 5}
- 15:44 `calib-longcat-2.5-preview-free-20261008-123322` completed=11/89 running=2 pass1.0=1 pass_rate=0.091 errored=5 rewards={'0.0': 10, '1.0': 1} exceptions={'AgentTimeoutError': 5}
- 16:16 `calib-longcat-2.5-preview-free-20261008-123322` completed=17/89 running=2 pass1.0=5 pass_rate=0.294 errored=7 rewards={'0.0': 11, '1.0': 5} exceptions={'AgentTimeoutError': 7}
- 16:47 `calib-longcat-2.5-preview-free-20261008-123322` completed=20/89 running=2 pass1.0=5 pass_rate=0.250 errored=10 rewards={'0.0': 14, '1.0': 5} exceptions={'AgentTimeoutError': 10}
- 17:06 `calib-longcat-2.5-preview-free-20261008-123322` completed=23/89 running=2 pass1.0=7 pass_rate=0.304 errored=11 rewards={'0.0': 15, '1.0': 7} exceptions={'AgentTimeoutError': 11}
- 17:19 `calib-longcat-2.5-preview-free-20261008-123322` completed=24/89 running=2 pass1.0=8 pass_rate=0.333 errored=11 rewards={'0.0': 15, '1.0': 8} exceptions={'AgentTimeoutError': 11}
- 17:32 `calib-longcat-2.5-preview-free-20261008-123322` completed=27/89 running=2 pass1.0=9 pass_rate=0.333 errored=13 rewards={'0.0': 17, '1.0': 9} exceptions={'AgentTimeoutError': 13}
- 17:50 `calib-longcat-2.5-preview-free-20261008-123322` completed=31/89 running=2 pass1.0=9 pass_rate=0.290 errored=13 rewards={'0.0': 21, '1.0': 9} exceptions={'AgentTimeoutError': 13}
- 17:56 `calib-longcat-2.5-preview-free-20261008-123322` completed=33/89 running=2 pass1.0=10 pass_rate=0.303 errored=13 rewards={'0.0': 22, '1.0': 10} exceptions={'AgentTimeoutError': 13}
- 18:21 `calib-longcat-2.5-preview-free-20261008-123322` completed=36/89 running=2 pass1.0=10 pass_rate=0.278 errored=13 rewards={'0.0': 25, '1.0': 10} exceptions={'AgentTimeoutError': 13}
- 18:21 `calib-longcat-2.5-preview-free-20261008-123322` completed=36/89 running=2 pass1.0=10 pass_rate=0.278 errored=13 rewards={'0.0': 25, '1.0': 10} exceptions={'AgentTimeoutError': 13}
- 18:45 `calib-longcat-2.5-preview-free-20261008-123322` completed=40/89 running=2 pass1.0=11 pass_rate=0.275 errored=15 rewards={'0.0': 28, '1.0': 11} exceptions={'AgentTimeoutError': 15}
- 18:53 `calib-longcat-2.5-preview-free-20261008-123322` completed=40/89 running=2 pass1.0=11 pass_rate=0.275 errored=15 rewards={'0.0': 28, '1.0': 11} exceptions={'AgentTimeoutError': 15}
- 19:10 `calib-longcat-2.5-preview-free-20261008-123322` completed=46/89 running=2 pass1.0=14 pass_rate=0.304 errored=17 rewards={'0.0': 30, '1.0': 14} exceptions={'AgentTimeoutError': 16, 'NonZeroAgentExitCodeError': 1}
- 19:25 `calib-longcat-2.5-preview-free-20261008-123322` completed=47/89 running=2 pass1.0=14 pass_rate=0.298 errored=18 rewards={'0.0': 31, '1.0': 14} exceptions={'AgentTimeoutError': 17, 'NonZeroAgentExitCodeError': 1}
- 19:34 `calib-longcat-2.5-preview-free-20261008-123322` completed=50/89 running=2 pass1.0=15 pass_rate=0.300 errored=19 rewards={'0.0': 33, '1.0': 15} exceptions={'AgentTimeoutError': 18, 'NonZeroAgentExitCodeError': 1}
- 19:57 `calib-longcat-2.5-preview-free-20261008-123322` completed=51/89 running=2 pass1.0=15 pass_rate=0.294 errored=19 rewards={'0.0': 34, '1.0': 15} exceptions={'AgentTimeoutError': 18, 'NonZeroAgentExitCodeError': 1}
- 19:59 `calib-longcat-2.5-preview-free-20261008-123322` completed=51/89 running=2 pass1.0=15 pass_rate=0.294 errored=19 rewards={'0.0': 34, '1.0': 15} exceptions={'AgentTimeoutError': 18, 'NonZeroAgentExitCodeError': 1}
- 20:23 `calib-longcat-2.5-preview-free-20261008-123322` completed=52/89 running=2 pass1.0=15 pass_rate=0.288 errored=19 rewards={'0.0': 35, '1.0': 15} exceptions={'AgentTimeoutError': 18, 'NonZeroAgentExitCodeError': 1}
- 20:29 `calib-longcat-2.5-preview-free-20261008-123322` completed=53/89 running=2 pass1.0=15 pass_rate=0.283 errored=19 rewards={'0.0': 36, '1.0': 15} exceptions={'AgentTimeoutError': 18, 'NonZeroAgentExitCodeError': 1}
- 20:48 `calib-longcat-2.5-preview-free-20261008-123322` completed=56/89 running=2 pass1.0=15 pass_rate=0.268 errored=20 rewards={'0.0': 39, '1.0': 15} exceptions={'AgentTimeoutError': 19, 'NonZeroAgentExitCodeError': 1}
- 21:01 `calib-longcat-2.5-preview-free-20261008-123322` completed=57/89 running=2 pass1.0=15 pass_rate=0.263 errored=20 rewards={'0.0': 40, '1.0': 15} exceptions={'AgentTimeoutError': 19, 'NonZeroAgentExitCodeError': 1}
- 21:13 `calib-longcat-2.5-preview-free-20261008-123322` completed=59/89 running=2 pass1.0=16 pass_rate=0.271 errored=21 rewards={'0.0': 41, '1.0': 16} exceptions={'AgentTimeoutError': 20, 'NonZeroAgentExitCodeError': 1}
- 21:33 `calib-longcat-2.5-preview-free-20261008-123322` completed=63/89 running=2 pass1.0=18 pass_rate=0.286 errored=23 rewards={'0.0': 43, '1.0': 18} exceptions={'AgentTimeoutError': 22, 'NonZeroAgentExitCodeError': 1}
- 21:37 `calib-longcat-2.5-preview-free-20261008-123322` completed=63/89 running=2 pass1.0=18 pass_rate=0.286 errored=23 rewards={'0.0': 43, '1.0': 18} exceptions={'AgentTimeoutError': 22, 'NonZeroAgentExitCodeError': 1}
- 22:02 `calib-longcat-2.5-preview-free-20261008-123322` completed=66/89 running=2 pass1.0=18 pass_rate=0.273 errored=23 rewards={'0.0': 46, '1.0': 18} exceptions={'AgentTimeoutError': 22, 'NonZeroAgentExitCodeError': 1}
- 22:05 `calib-longcat-2.5-preview-free-20261008-123322` completed=66/89 running=2 pass1.0=18 pass_rate=0.273 errored=23 rewards={'0.0': 46, '1.0': 18} exceptions={'AgentTimeoutError': 22, 'NonZeroAgentExitCodeError': 1}
- 22:27 `calib-longcat-2.5-preview-free-20261008-123322` completed=69/89 running=2 pass1.0=18 pass_rate=0.261 errored=24 rewards={'0.0': 48, '1.0': 18} exceptions={'AgentTimeoutError': 23, 'NonZeroAgentExitCodeError': 1}
- 22:37 `calib-longcat-2.5-preview-free-20261008-123322` completed=70/89 running=2 pass1.0=18 pass_rate=0.257 errored=25 rewards={'0.0': 49, '1.0': 18} exceptions={'AgentTimeoutError': 24, 'NonZeroAgentExitCodeError': 1}
- 22:52 `calib-longcat-2.5-preview-free-20261008-123322` completed=71/89 running=2 pass1.0=18 pass_rate=0.254 errored=25 rewards={'0.0': 50, '1.0': 18} exceptions={'AgentTimeoutError': 24, 'NonZeroAgentExitCodeError': 1}
- 23:09 `calib-longcat-2.5-preview-free-20261008-123322` completed=75/89 running=2 pass1.0=20 pass_rate=0.267 errored=25 rewards={'0.0': 52, '1.0': 20} exceptions={'AgentTimeoutError': 24, 'NonZeroAgentExitCodeError': 1}
- 23:16 `calib-longcat-2.5-preview-free-20261008-123322` completed=75/89 running=2 pass1.0=20 pass_rate=0.267 errored=25 rewards={'0.0': 52, '1.0': 20} exceptions={'AgentTimeoutError': 24, 'NonZeroAgentExitCodeError': 1}
- 23:41 `calib-longcat-2.5-preview-free-20261008-123322` completed=79/89 running=2 pass1.0=21 pass_rate=0.266 errored=29 rewards={'0.0': 55, '1.0': 21} exceptions={'AgentTimeoutError': 28, 'NonZeroAgentExitCodeError': 1}
- 23:41 `calib-longcat-2.5-preview-free-20261008-123322` completed=79/89 running=2 pass1.0=21 pass_rate=0.266 errored=29 rewards={'0.0': 55, '1.0': 21} exceptions={'AgentTimeoutError': 28, 'NonZeroAgentExitCodeError': 1}
- 00:06 `calib-longcat-2.5-preview-free-20261008-123322` completed=82/89 running=2 pass1.0=22 pass_rate=0.268 errored=29 rewards={'0.0': 57, '1.0': 22} exceptions={'AgentTimeoutError': 28, 'NonZeroAgentExitCodeError': 1}
- 00:14 `calib-longcat-2.5-preview-free-20261008-123322` completed=83/89 running=2 pass1.0=23 pass_rate=0.277 errored=29 rewards={'0.0': 57, '1.0': 23} exceptions={'AgentTimeoutError': 28, 'NonZeroAgentExitCodeError': 1}
- 00:31 `calib-longcat-2.5-preview-free-20261008-123322` completed=85/89 running=2 pass1.0=24 pass_rate=0.282 errored=30 rewards={'0.0': 58, '1.0': 24} exceptions={'AgentTimeoutError': 29, 'NonZeroAgentExitCodeError': 1}
- 00:46 `calib-longcat-2.5-preview-free-20261008-123322` completed=87/89 running=2 pass1.0=24 pass_rate=0.276 errored=31 rewards={'0.0': 59, '1.0': 24} exceptions={'AgentTimeoutError': 29, 'NonZeroAgentExitCodeError': 2}
- 00:56 `calib-longcat-2.5-preview-free-20261008-123322` completed=88/89 running=1 pass1.0=24 pass_rate=0.273 errored=31 rewards={'0.0': 60, '1.0': 24} exceptions={'AgentTimeoutError': 29, 'NonZeroAgentExitCodeError': 2}
- 01:02 `calib-longcat-2.5-preview-free-20261008-123322` **FINAL** completed=89/89 running=0 pass1.0=24 pass_rate=0.2697 errored=32

## Resultado final (Ronda 2 — Terminal-Bench 2 completo)

- **Job**: `jobs/calib-longcat-2.5-preview-free-20261008-123322/2026-10-08__12-33-23`
- **result.json**: `jobs/calib-longcat-2.5-preview-free-20261008-123322/2026-10-08__12-33-23/result.json`
- **pass_rate (mean)**: **0.2697** (24/89) · `n_total_trials=89` · `n_completed_trials=89` · `n_errored_trials=32`
- **Runtime**: 12h 24m 37s · `--n-concurrent 2` · guard `MAX_REQ=4000` (0 aborts)
- **Excepciones**: `AgentTimeoutError=30`, `NonZeroAgentExitCodeError=2`
- **reward 1.0 = 24** · **reward 0.0 = 61** · (4 errored sin entry de reward)
- **Secreto**: clave redactada (`REDACTED`) en 89 `config.json`; `LEAK=NO`.

### Tareas aprobadas (24)
`constraints-scheduling, crack-7z-hash, db-wal-recovery, distribution-search, fix-code-vulnerability, git-leak-recovery, git-multibranch, hf-model-inference, kv-store-grpc, large-scale-text-editing, log-summary-date-ranges, mcmc-sampling-stan, modernize-scientific-stack, multi-source-data-merger, nginx-request-logging, openssl-selfsigned-cert, portfolio-optimization, prove-plus-comm, pypi-server, rstan-to-pystan, sparql-university, sqlite-db-truncate, sqlite-with-gcov, vulnerable-secret`
