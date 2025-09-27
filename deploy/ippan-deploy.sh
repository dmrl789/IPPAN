#!/usr/bin/env bash
set -euo pipefail

COMPOSE_PATH="${COMPOSE_REMOTE_PATH:-/opt/ippan/docker-compose.yml}"
COMPOSE_DIR="$(dirname "$COMPOSE_PATH")"

log() {
  printf '[deploy] %s\n' "$*"
}

if [[ ! -f "$COMPOSE_PATH" ]]; then
  log "compose file '$COMPOSE_PATH' not found"
  exit 1
fi

if command -v docker >/dev/null 2>&1; then
  DOCKER_BIN="docker"
else
  log "docker is not installed on the target host"
  exit 1
fi

if $DOCKER_BIN compose version >/dev/null 2>&1; then
  COMPOSE_CMD=($DOCKER_BIN compose)
elif command -v docker-compose >/dev/null 2>&1; then
  COMPOSE_CMD=(docker-compose)
else
  log "docker compose plugin or docker-compose binary not available"
  exit 1
fi

if [[ -n "${GHCR_TOKEN:-}" ]]; then
  log "logging in to ghcr.io as '${GHCR_USER:-github}'"
  printf '%s\n' "$GHCR_TOKEN" | $DOCKER_BIN login ghcr.io \
    --username "${GHCR_USER:-github}" --password-stdin
else
  log "GHCR_TOKEN not provided; skipping registry login"
fi

cd "$COMPOSE_DIR"

log "pulling images defined in $(basename "$COMPOSE_PATH")"
"${COMPOSE_CMD[@]}" -f "$COMPOSE_PATH" pull

log "applying updated stack"
"${COMPOSE_CMD[@]}" -f "$COMPOSE_PATH" up -d --remove-orphans

log "removing dangling images"
$DOCKER_BIN image prune -f --filter "dangling=true" || log "image prune skipped"

log "deployment completed successfully"
