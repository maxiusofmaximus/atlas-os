#!/usr/bin/env bash
set -e
# Install the Docker Compose v2 plugin (docker.io ships only the CLI + a few plugins).
mkdir -p /usr/local/lib/docker/cli-plugins
ARCH=$(uname -m)
case "$ARCH" in
  x86_64) CARCH=x86_64 ;;
  aarch64) CARCH=aarch64 ;;
  *) echo "unsupported arch $ARCH"; exit 1 ;;
esac
VER=$(curl -sL https://api.github.com/repos/docker/compose/releases/latest | grep -oP '"tag_name":\s*"\K[^"]+')
echo "compose version: $VER ($CARCH)"
curl -sSL "https://github.com/docker/compose/releases/download/${VER}/docker-compose-linux-${CARCH}" \
  -o /usr/local/lib/docker/cli-plugins/docker-compose
chmod +x /usr/local/lib/docker/cli-plugins/docker-compose
docker compose version
