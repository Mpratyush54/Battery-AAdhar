#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "ERROR missing required command: $1" >&2
    exit 1
  fi
}

require_cmd curl
require_cmd docker

echo "==> Smoke test: checking compose services"
docker compose -f "$ROOT_DIR/docker-compose.yaml" ps

echo "==> Smoke test: API health"
curl -fsS http://localhost:8080/healthz >/dev/null
echo "OK /healthz"

echo "==> Smoke test: API readiness"
curl -fsS http://localhost:8080/readyz >/dev/null
echo "OK /readyz"

echo "==> Smoke test passed"
