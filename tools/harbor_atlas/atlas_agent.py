"""Harbor installed-agent adapter for Atlas OS (RFC 20 Fase 22, EVAL.3).

Dev-only integration scaffold: it is NOT part of the Atlas single-binary
distribution (RFC 25 §11). It requires Harbor + Docker + a model endpoint.

Register it with Harbor (from the repo root, with `PYTHONPATH=tools`):

    harbor run -d terminal-bench@2.0 \\
        --agent harbor_atlas.atlas_agent:AtlasAgent \\
        --model <provider/model>

The `run` body is the integration point: point it at the Atlas headless
entrypoint you want evaluated. Ingest the results with
`atlas eval import jobs/<job-id>` (see README.md).
"""

import shlex

from harbor.agents.installed.base import BaseInstalledAgent, with_prompt_template
from harbor.environments.base import BaseEnvironment
from harbor.models.agent.context import AgentContext


class AtlasAgent(BaseInstalledAgent):
    """Runs the Atlas OS CLI headlessly inside the task container."""

    def name(self) -> str:
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
        await self.exec_as_agent(
            environment,
            command=(
                "atlas --profile harbor mission new " + prompt + " && "
                "atlas --profile harbor run"
            ),
        )

    def populate_context_post_run(self, context: AgentContext) -> None:
        return None
