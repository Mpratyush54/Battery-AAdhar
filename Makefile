# ═══════════════════════════════════════════════════════════════════════════
#  Battery Aadhaar — top-level Makefile
#  Usage: make <target>
# ═══════════════════════════════════════════════════════════════════════════

.PHONY: help proto build-rust build-go test test-rust test-go \
        docker-up docker-down docker-logs clean lint lint-rust lint-go \
        check migrate

# Default target
.DEFAULT_GOAL := help

RUST_DIR  := core
GO_DIR    := api
PROTO_DIR := proto

# ── Help ───────────────────────────────────────────────────────────────────

help:
	@echo ""
	@echo "  Battery Aadhaar — available targets"
	@echo "  ────────────────────────────────────"
	@echo "  proto        Generate proto stubs (Go + buf lint)"
	@echo "  build-rust   cargo build --release in core/"
	@echo "  build-go     go build ./... in api/"
	@echo "  test         Run ALL tests (Rust + Go)"
	@echo "  test-rust    cargo test in core/"
	@echo "  test-go      go test ./... in api/"
	@echo "  lint         Run all linters"
	@echo "  lint-rust    cargo clippy in core/"
	@echo "  lint-go      golangci-lint in api/"
	@echo "  docker-up    docker compose up -d (builds if needed)"
	@echo "  docker-down  docker compose down"
	@echo "  docker-logs  docker compose logs -f"
	@echo "  migrate      Run DB migrations (golang-migrate)"
	@echo "  clean        Remove build artefacts"
	@echo ""

# ── Proto ──────────────────────────────────────────────────────────────────

proto:
	@echo "→ Linting proto files..."
	buf lint $(PROTO_DIR)
	@echo "→ Generating Go stubs..."
	buf generate
	@echo "✓ proto done"

# ── Rust ───────────────────────────────────────────────────────────────────

build-rust:
	@echo "→ Building Rust (release)..."
	cd $(RUST_DIR) && cargo build --release
	@echo "✓ Rust build done"

test-rust:
	@echo "→ Testing Rust..."
	cd $(RUST_DIR) && cargo test -- --test-threads=4
	@echo "✓ Rust tests done"

lint-rust:
	@echo "→ Linting Rust..."
	cd $(RUST_DIR) && cargo clippy -- -D warnings
	@echo "✓ Rust lint done"

# ── Go ─────────────────────────────────────────────────────────────────────

build-go:
	@echo "→ Building Go..."
	cd $(GO_DIR) && go build ./...
	@echo "✓ Go build done"

test-go:
	@echo "→ Testing Go..."
	cd $(GO_DIR) && go test -race -coverprofile=coverage.out ./...
	@echo "✓ Go tests done"

lint-go:
	@echo "→ Linting Go..."
	cd $(GO_DIR) && golangci-lint run ./...
	@echo "✓ Go lint done"

# ── Combined targets ───────────────────────────────────────────────────────

build: build-rust build-go

test: test-rust test-go

lint: lint-rust lint-go

# ── Docker ─────────────────────────────────────────────────────────────────

docker-up:
	@echo "→ Starting stack..."
	docker compose up -d --build
	@echo "✓ Stack up. Postgres: 5432 | Redis: 6379 | Rust gRPC: 50051 | Go API: 8080"

docker-down:
	@echo "→ Stopping stack..."
	docker compose down
	@echo "✓ Stack down"

docker-logs:
	docker compose logs -f

# ── DB migrations ──────────────────────────────────────────────────────────

migrate:
	@echo "→ Running DB migrations..."
	migrate -path api/migrations \
	        -database "$$DATABASE_URL" \
	        up
	@echo "✓ Migrations applied"

# ── Clean ──────────────────────────────────────────────────────────────────

clean:
	cd $(RUST_DIR) && cargo clean
	cd $(GO_DIR) && go clean ./...
	rm -rf api/gen/proto/*.pb.go
	@echo "✓ Clean done"
