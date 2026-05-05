# Battery Pack Aadhaar (BPA) Core Engine

The BPA Core Engine is a high-performance gRPC service built in Rust that manages lifecycle, compliance, and regulatory tracking for batteries.

## Quick Start (Phase 1)

Run from repo root: `D:\Battery`.

### Prerequisites

- Rust toolchain
- Go toolchain
- `buf` CLI
- Docker Desktop (for compose smoke and full demo)
- On Windows for `go test -race`: MinGW-w64 `gcc` on `PATH`

### Lint and Tests

```bash
cd core
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --lib -- --nocapture --test-threads=1
cargo test --test integration_test -- --nocapture

cd ../api
go fmt ./...
go vet ./...
staticcheck ./...
go test -race ./...

cd ..
./buf.exe lint .
```

### Docker Smoke

```bash
docker compose up -d --build
./scripts/smoke-test.sh
```

### Demo Flow

```bash
./scripts/phase1-demo.sh
```

### Tear Down

```bash
docker compose down
```

## mTLS Setup (Rust <-> Go)

Generate certificates:

```bash
./scripts/generate-certs.sh
```

Expected files:
- `certs/ca.crt`
- `certs/server.crt`
- `certs/server.key`
- `certs/client.crt`
- `certs/client.key`

## Project Layout

- `core/`: Rust gRPC services and crypto domain
- `api/`: Go HTTP API, middleware, and gRPC clients
- `proto/`: protobuf definitions
- `scripts/`: helper scripts (cert generation, smoke test, demo)
- `docs/`: architecture and phase completion docs
