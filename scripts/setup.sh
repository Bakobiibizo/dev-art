#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

NO_CACHE=0
for arg in "$@"; do
  case "$arg" in
    --no-cache)
      NO_CACHE=1
      ;;
    -h|--help)
      echo "Usage: $0 [--no-cache]" >&2
      exit 0
      ;;
    *)
      echo "[setup] error: unknown argument: $arg" >&2
      echo "Usage: $0 [--no-cache]" >&2
      exit 2
      ;;
  esac
done

log() {
  echo "[setup] $*"
}

die() {
  echo "[setup] error: $*" >&2
  exit 1
}

command -v docker >/dev/null 2>&1 || die "docker is required"
docker compose version >/dev/null 2>&1 || die "docker compose plugin is required (docker compose ...)"

log "Building container (dev-art)..."
(
  cd "$ROOT_DIR"
  if [[ "$NO_CACHE" == "1" ]]; then
    docker compose build --no-cache dev-art
  else
    docker compose build dev-art
  fi
)

log "Starting container (dev-art)..."
(
  cd "$ROOT_DIR"
  docker compose up -d dev-art
)

log "Done. Useful commands:"
log "  docker compose logs -f dev-art"
log "  docker compose exec dev-art bash"