#!/usr/bin/env bash
export PATH="$HOME/.local/bin:$PATH"
ATLAS="/mnt/c/Users/Max/Desktop/atlas-os/src-tauri/target/release/atlas"
ENVF="/mnt/c/Users/Max/Desktop/atlas-os/.env"

echo "=== run atlas inside a clean ubuntu container, same mounts ==="
docker run --rm \
  -v "$ATLAS:/usr/local/bin/atlas:ro" \
  -v "$ENVF:/app/.env:ro" \
  -w /app \
  ubuntu:24.04 \
  bash -lc 'atlas --version; echo "--- mission ---"; atlas --profile harbor mission new --force "write hello to /app/hi.txt"; echo "EXIT=$?"'
