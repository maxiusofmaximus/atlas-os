#!/usr/bin/env bash
set -e
export PATH="$HOME/.local/bin:$PATH"
if ! command -v uv >/dev/null 2>&1; then
  curl -LsSf https://astral.sh/uv/install.sh | sh >/dev/null 2>&1
fi
export PATH="$HOME/.local/bin:$PATH"
echo "uv: $(uv --version)"
echo "--- installing harbor ---"
uv tool install harbor 2>&1 | tail -n 5
export PATH="$HOME/.local/bin:$PATH"
echo "--- harbor version ---"
harbor --version 2>&1 || harbor version 2>&1 || echo "(no version subcmd)"
