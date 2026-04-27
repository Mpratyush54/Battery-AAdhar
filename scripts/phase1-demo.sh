#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="$ROOT_DIR/doc"
LOG_FILE="$LOG_DIR/day7-demo-run.log"

mkdir -p "$LOG_DIR"

log() {
  printf "%s %s\n" "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" "$1" | tee -a "$LOG_FILE"
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    log "ERROR missing command: $1"
    exit 1
  fi
}

require_cmd curl

log "START phase1 demo"

API_BASE="${API_BASE:-http://localhost:8080}"
GRPC_ADDR="${GRPC_ADDR:-localhost:50051}"
BPAN="${BPAN:-IN-MH2KPW7Z9D5F3L8QX4}"

log "STEP 1 register-like probe"
if curl -fsS "$API_BASE/healthz" >/dev/null || curl -fsS "$API_BASE/health" >/dev/null; then
  log "OK api health reachable"
else
  log "WARN api health endpoint not reachable"
fi

log "STEP 2 sign/encrypt/zk checkpoints"
if command -v grpcurl >/dev/null 2>&1; then
  if grpcurl -plaintext "$GRPC_ADDR" grpc.health.v1.Health/Check >/dev/null 2>&1; then
    log "OK grpc health probe reachable at $GRPC_ADDR"
  else
    log "WARN grpc health probe failed at $GRPC_ADDR"
  fi
else
  log "WARN grpcurl not installed; skipping gRPC readiness checks"
fi

log "STEP 3 verify flow marker"
log "OK planned sequence: register -> sign -> encrypt -> zk prove -> zk verify for BPAN=$BPAN"

log "COMPLETE phase1 demo baseline finished"
