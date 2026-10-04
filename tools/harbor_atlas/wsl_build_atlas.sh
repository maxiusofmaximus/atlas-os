#!/usr/bin/env bash
set -e
export PATH="$HOME/.local/bin:$PATH"
cd /mnt/c/Users/Max/Desktop/atlas-os
echo "=== rustup? ==="
command -v cargo >/dev/null 2>&1 || {
  curl -LsSf https://sh.rustup.rs | sh -s -- -y --profile minimal >/dev/null 2>&1
  . "$HOME/.cargo/env"
}
cargo --version
echo "=== building atlas (release, linux) ==="
cargo build --release --manifest-path src-tauri/Cargo.toml --bin atlas 2>&1 | tail -n 3
echo "=== binary ==="
file src-tauri/target/release/atlas
ls -la src-tauri/target/release/atlas
echo "=== smoke: atlas --help ==="
src-tauri/target/release/atlas --help 2>&1 | head -5
