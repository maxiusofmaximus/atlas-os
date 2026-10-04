"""Harbor installed-agent adapter for Atlas OS (RFC 20 Fase 25/39; research/54, research/60).

Dev-only integration: NOT part of the Atlas single-binary distribution (RFC 25 §11).
Requires Harbor + Docker + a model endpoint and a `harbor` Atlas profile with a
matching deployment/API key.

Register it with Harbor (from the repo root, with `PYTHONPATH=tools`):

    harbor run -d terminal-bench/terminal-bench-2 \\
        --agent harbor_atlas.atlas_agent:AtlasAgent \\
        --model <provider/model>

Two modes (env `ATLAS_AGENT_MODE`):
- `agent`  (default): the terminal agent loop (`atlas agent "<instruction>"`) — the
  model runs shell commands in the task's working dir. This is the mode that can
  actually solve Terminal-Bench tasks (Fase 39).
- `pipeline`: the orchestrated mission→plan→execute --coding chain (Fase 25).
"""

import os
import shlex

from harbor.agents.installed.base import BaseInstalledAgent, with_prompt_template
from harbor.environments.base import BaseEnvironment
from harbor.models.agent.context import AgentContext


class AtlasAgent(BaseInstalledAgent):
    """Runs Atlas OS headlessly in the task container."""

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
        mode = os.environ.get("ATLAS_AGENT_MODE", "agent").strip().lower()
        profile = os.environ.get("ATLAS_PROFILE", "harbor")

        if mode == "pipeline":
            # Orchestrated: mission (force-lock) → plan → execute --coding --apply.
            command = (
                "ID=$(atlas --profile " + profile + " mission new --force "
                + prompt
                + " | awk '/^mission /{print $2; exit}') && "
                'atlas --profile ' + profile + ' plan "$ID" && '
                "for i in 1 2 3 4 5; do "
                'atlas --profile ' + profile + ' execute --coding --apply --root . "$ID" && break; '
                'echo "execute attempt $i failed (transient?), retrying in $((i*10))s"; '
                "sleep $((i*10)); "
                "done"
            )
        else:
            # Terminal agent loop: the model runs shell commands to solve the task.
            # Retry the whole loop on a transient provider error (rate-limit).
            command = (
                "for i in 1 2 3 4 5; do "
                "atlas --profile " + profile + " agent --root . --max-steps 40 " + prompt
                + ' && break; '
                'echo "agent attempt $i failed (transient?), retrying in $((i*15))s"; '
                "sleep $((i*15)); "
                "done"
            )
        await self.exec_as_agent(environment, command=command)

    def populate_context_post_run(self, context: AgentContext) -> None:
        return None
