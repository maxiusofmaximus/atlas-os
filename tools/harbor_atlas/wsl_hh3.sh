#!/usr/bin/env bash
export PATH="$HOME/.local/bin:$PATH"
harbor run --help 2>&1 | sed -n '185,260p'
