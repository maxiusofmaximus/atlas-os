"""Harbor installed-agent adapter for Atlas OS (RFC 20 Fase 25; research/54).

Dev-only integration scaffold: it is NOT part of the Atlas single-binary
distribution (RFC 25 §11). It requires Harbor + Docker + a model endpoint and a
`harbor` Atlas profile with a matching deployment/API key.

Register it with Harbor (from the repo root, with `PYTHONPATH=tools`):

    harbor run -d terminal-bench@2.0 \\
        --agent harbor_atlas.atlas_agent:AtlasAgent \\
        --model <provider/model>

The agent drives the REAL Atlas pipeline — mission → plan → execute — so the
resulting benchmark number measures the orchestrated harness (routing + cascade
+ reliability gate + cost guard), not the Phase-1 single pass.
"""

import shlex

from harbor.agents.installed.base import BaseInstalledAgent, with_prompt_template
from harbor.environments.base import BaseEnvironment
from harbor.models.agent.context import AgentContext


class AtlasAgent(BaseInstalledAgent):
    """Runs the Atlas OS orchestrated pipeline headlessly in the container."""

    @staticmethod
    def name() -> str:
        return "atlas"

    async def install(self, environment: BaseEnvironment) -> None:
        await self.exec_as_root(
            environment,
            command="apt-get update && apt-get install -y curl ca-certificates",
        )

    @with_prompt_template
    async def run(
        self, instruction: str, environment: BaseEnvironment, context: AgentContext
    ) -> None:
        prompt = shlex.quote(instruction)
        # 1) force-lock the mission (offline heuristic planner; `--force` overrides
        #    the auto-lock confidence threshold so the pipeline never stalls),
        # 2) generate the Plan, 3) drive it through the orchestrator's real coding
        #    loop (structured Diff → Validation → Repair → apply to the workspace).
        # `&&` makes any failing stage fail the trial. `execute` retries on a
        # transient "cascade exhausted ... 0 attempts" (provider rate-limit):
        # without it a single 429 in a concurrent sweep fails the whole trial.
        command = (
            "ID=$(atlas --profile harbor mission new --force "
            + prompt
            + " | awk '/^mission /{print $2; exit}') && "
            'atlas --profile harbor plan "$ID" && '
            'for i in 1 2 3 4 5; do '
            'atlas --profile harbor execute --coding --apply --root . "$ID" && break; '
            'echo "execute attempt $i failed (transient?), retrying in $((i*10))s"; '
            'sleep $((i*10)); '
            "done"
        )
        await self.exec_as_agent(environment, command=command)

    def populate_context_post_run(self, context: AgentContext) -> None:
        return None
