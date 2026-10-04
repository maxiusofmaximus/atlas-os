#!/usr/bin/env bash
export PATH="$HOME/.local/bin:$PATH"
python3 - <<'PY'
from harbor.models.agent.context import AgentContext
print("fields:", list(AgentContext.model_fields.keys()))
PY
