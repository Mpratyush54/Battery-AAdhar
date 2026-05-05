<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.78+-orange?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/Go-1.22+-00ADD8?logo=go&logoColor=white" alt="Go" />
  <img src="https://img.shields.io/badge/gRPC-Protobuf_v3-4285F4?logo=google&logoColor=white" alt="gRPC" />
  <img src="https://img.shields.io/badge/License-MIT-green" alt="License" />
  <img src="https://img.shields.io/badge/Docker-Compose_v3.8-2496ED?logo=docker&logoColor=white" alt="Docker" />
</p>

<h1 align="center">🔋 Battery Pack Aadhaar (BPA)</h1>

<p align="center">
  <strong>A Zero-Knowledge Battery Authentication &amp; Lifecycle Platform</strong><br/>
  <em>Digital identity, provenance tracking, and compliance enforcement for every battery — from manufacturing to recycling.</em>
</p>

---

## 📋 Table of Contents

- [Overview](#-overview)
- [Key Features](#-key-features)
- [Architecture](#-architecture)
- [Tech Stack](#-tech-stack)
- [Project Structure](#-project-structure)
- [Prerequisites](#-prerequisites)
- [Quick Start](#-quick-start)
- [Configuration](#-configuration)
- [API Reference](#-api-reference)
- [Testing](#-testing)
- [Docker Deployment](#-docker-deployment)
- [Semantic Versioning](#-semantic-versioning)
- [Contributing](#-contributing)
- [Roadmap](#-roadmap)
- [License](#-license)

---

## 🔍 Overview

**Battery Pack Aadhaar (BPA)** assigns every battery pack a unique, tamper-proof digital identity — the **Battery Pack Aadhaar Number (BPAN)** — enabling full lifecycle traceability across manufacturing, distribution, operation, second-life reuse, and recycling.

The platform is built on a **dual-service architecture**:

| Layer | Language | Role |
|-------|----------|------|
| **Core Engine** | Rust | Cryptographic operations, ZK proofs, gRPC services, data integrity |
| **API Gateway** | Go | REST API, JWT/RBAC auth, request routing, Swagger docs |

Communication between the two layers is secured via **mTLS-authenticated gRPC**, with secrets managed through **Infisical** or local `.env` files.

---

## ✨ Key Features

### 🔐 Security & Cryptography
- **3-Tier Key Hierarchy**: Root Key → KEK → DEK with AES-256-GCM encryption
- **Zero-Knowledge Proofs**: Bulletproofs-based private State-of-Health (SoH) verification
- **mTLS gRPC**: Mutual TLS between Go gateway and Rust core
- **JWT + RBAC**: Role-based access control (Manufacturer, Regulator, Recycler, End-user)
- **Tamper-Evident Audit Chain**: SHA-256 hash-linked append-only audit log

### 🔋 Battery Lifecycle
- **BPAN Codec**: Unique 21-char identifier encoding country, chemistry, capacity, and manufacturer
- **Registration Pipeline**: Atomic battery onboarding with descriptor validation and integrity hashing
- **Health Monitoring (BDD)**: Real-time SoH tracking with automatic lifecycle state transitions
- **Carbon Footprint (BCF)**: 5-stage emissions model (Mining → Manufacturing → Transport → Operation → Recycling)

### 🏗️ Infrastructure
- **Dockerized Stack**: Postgres 15, Redis 7, Rust gRPC, Go API — all orchestrated via `docker-compose`
- **CI/CD Pipeline**: GitHub Actions with proto lint, Rust clippy/test/audit, Go vet/staticcheck/test
- **Swagger/OpenAPI**: Auto-generated API documentation
- **Degraded Mode**: Graceful fallback when external dependencies are unavailable

---

## 🏛️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Client / Browser                       │
└────────────────────────┬────────────────────────────────────┘
                         │ HTTPS (REST)
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   Go API Gateway (:8080)                    │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐  ┌───────────┐ │
│  │   Auth   │  │  Battery │  │  Carbon   │  │  Health   │ │
│  │Controller│  │Controller│  │Controller │  │Controller │ │
│  └────┬─────┘  └────┬─────┘  └─────┬─────┘  └─────┬─────┘ │
│       │   JWT/RBAC Middleware       │              │        │
│  ┌────┴─────────────┴───────────────┴──────────────┴─────┐ │
│  │              gRPC Client Factory (mTLS)                │ │
│  └───────────────────────┬───────────────────────────────┘ │
└──────────────────────────┼──────────────────────────────────┘
                           │ gRPC + mTLS (:50051)
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                  Rust Core Engine (:50051)                   │
│  ┌────────────┐  ┌────────────┐  ┌────────────────────────┐│
│  │ BPA Engine │  │ ZK Prover  │  │   Key Manager (KMS)    ││
│  │ (Battery   │  │(Bulletproof│  │ Root → KEK → DEK       ││
│  │ Register)  │  │  SoH)      │  │ AES-256-GCM            ││
│  └─────┬──────┘  └──────┬─────┘  └────────────┬───────────┘│
│        │                │                      │            │
│  ┌─────┴────────────────┴──────────────────────┴───────────┐│
│  │                  Repository Layer (sqlx)                 ││
│  └─────────────────────────┬───────────────────────────────┘│
└────────────────────────────┼────────────────────────────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
        ┌──────────┐  ┌──────────┐   ┌──────────┐
        │ Postgres │  │  Redis   │   │ Infisical│
        │  (Data)  │  │ (Cache)  │   │ (Secrets)│
        └──────────┘  └──────────┘   └──────────┘
```

---

## 🛠️ Tech Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| Core Engine | Rust 1.78+ | Cryptography, gRPC services, data integrity |
| API Gateway | Go 1.22+ | REST API, auth, middleware |
| RPC Framework | gRPC + Protobuf v3 | Inter-service communication |
| Database | PostgreSQL 15 | Persistent storage |
| Cache | Redis 7 | Session & rate-limit cache |
| Secret Manager | Infisical | Credential management |
| ZK Proofs | Bulletproofs + Merlin | Privacy-preserving verification |
| Encryption | AES-256-GCM, Ed25519 | Data encryption & signing |
| Auth | JWT + bcrypt | Authentication & password hashing |
| CI/CD | GitHub Actions | Automated testing & audit |
| Containerization | Docker Compose v3.8 | Multi-service orchestration |
| API Docs | Swagger / OpenAPI | Auto-generated documentation |
| Proto Lint | Buf CLI | Protobuf linting & breaking-change detection |

---

## 📂 Project Structure

```
Battery/
├── core/                       # Rust gRPC core engine
│   ├── src/
│   │   ├── main.rs             # Server bootstrap (gRPC + HTTP)
│   │   ├── lib.rs              # Library root
│   │   ├── errors.rs           # Error types
│   │   ├── api/                # gRPC service handlers
│   │   │   └── battery.rs      # RegisterBattery RPC
│   │   ├── models/             # Domain models
│   │   │   ├── battery.rs      # Battery descriptor & BPAN
│   │   │   ├── health.rs       # Health status & SoH tracking
│   │   │   └── carbon.rs       # Carbon footprint models
│   │   ├── services/           # Business logic
│   │   │   ├── key_manager.rs  # 3-tier KMS (Root/KEK/DEK)
│   │   │   ├── zk_proofs.rs    # Bulletproofs SoH prover
│   │   │   ├── health.rs       # Health monitoring service
│   │   │   └── carbon.rs       # Carbon emissions engine
│   │   └── repositories/       # Database access layer
│   │       ├── battery_repo.rs # Battery CRUD
│   │       └── audit_repo.rs   # Append-only audit chain
│   ├── tests/                  # Integration tests
│   ├── Cargo.toml
│   └── build.rs                # Protobuf codegen
│
├── api/                        # Go REST API gateway
│   ├── main.go                 # HTTP server entry point
│   ├── controllers/            # HTTP route handlers
│   ├── middleware/              # Auth, RBAC, logging
│   ├── services/               # Business logic wrappers
│   ├── config/                 # DB, Redis, gRPC config
│   ├── grpc/                   # gRPC client factory
│   ├── bpan/                   # BPAN codec (encode/decode)
│   ├── routes/                 # HTTP router setup
│   ├── models/                 # Request/response DTOs
│   ├── docs/                   # Swagger auto-generated
│   └── tests/                  # Integration & E2E tests
│
├── proto/                      # Protobuf definitions
│   ├── battery.proto           # Battery registration & lookup
│   ├── crypto.proto            # Encrypt/Decrypt/Sign RPCs
│   ├── auth.proto              # Authentication messages
│   ├── lifecycle.proto         # Lifecycle state transitions
│   └── common.proto            # Shared types
│
├── scripts/                    # Automation scripts
│   ├── generate-certs.sh       # mTLS certificate generation
│   ├── smoke-test.sh           # Docker smoke test
│   └── phase1-demo.sh          # End-to-end demo
│
├── certs/                      # TLS certificates (gitignored)
├── doc/                        # Design docs & LinkedIn posts
├── .github/workflows/ci.yml    # CI pipeline
├── docker-compose.yaml         # Production stack
├── docker-compose.test.yaml    # Test stack
├── Makefile                    # Build automation
├── buf.yaml                    # Buf proto config
└── buf.gen.yaml                # Buf code generation config
```

---

## 📦 Prerequisites

| Tool | Version | Required For |
|------|---------|-------------|
| [Rust](https://rustup.rs/) | 1.78+ | Core engine |
| [Go](https://go.dev/dl/) | 1.22+ | API gateway |
| [Docker](https://docs.docker.com/get-docker/) | 24+ | Container orchestration |
| [Buf CLI](https://buf.build/docs/installation) | Latest | Protobuf linting & codegen |
| [Make](https://www.gnu.org/software/make/) | Any | Build automation |
| MinGW-w64 gcc | Any | Windows only: `go test -race` |

---

## 🚀 Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/Mpratyush54/Battery-AAdhar.git
cd Battery-AAdhar
```

### 2. Start with Docker (Recommended)

```bash
# Start the full stack (Postgres, Redis, Rust Core, Go API)
docker compose up -d --build

# Verify all services are running
docker compose ps

# Run smoke test
./scripts/smoke-test.sh

# Access API docs
open http://localhost:8080/swagger/index.html
```

### 3. Local Development (Without Docker)

```bash
# ── Rust Core ──
cd core
cargo build --release
cargo run  # Starts gRPC server on :50051

# ── Go API (in a separate terminal) ──
cd api
go run main.go  # Starts REST API on :8080
```

### 4. Generate mTLS Certificates

```bash
./scripts/generate-certs.sh
```

This creates:
- `certs/ca.crt` — Certificate Authority
- `certs/server.crt` / `server.key` — Rust core server
- `certs/client.crt` / `client.key` — Go gateway client

---

## ⚙️ Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `postgres://bpa_user:bpa_pass@localhost:5432/bpa_db` | PostgreSQL connection string |
| `REDIS_HOST` | `localhost` | Redis hostname |
| `REDIS_PORT` | `6379` | Redis port |
| `REDIS_PASSWORD` | *(empty)* | Redis password |
| `GRPC_TARGET` | `localhost:50051` | Rust gRPC service address |
| `ENCRYPTION_KEY` | — | 32-byte AES-256 key |
| `RUST_LOG` | `info` | Rust log level |
| `PORT` | `8080` | Go API listen port |
| `GRPC_CA_CERT_PEM` | — | CA cert PEM (inline or via Infisical) |
| `GRPC_CLIENT_CERT_PEM` | — | Client cert PEM |
| `GRPC_CLIENT_KEY_PEM` | — | Client key PEM |
| `INFISICAL_CLIENT_ID` | — | Infisical auth client ID |
| `INFISICAL_CLIENT_SECRET` | — | Infisical auth client secret |
| `INFISICAL_PROJECT_ID` | — | Infisical project ID |
| `INFISICAL_ENV` | `dev` | Infisical environment |
| `BPA_ALLOW_DB_FAILURE` | `0` | Set to `1` for degraded mode (CI) |

---

## 📡 API Reference

### Authentication

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/auth/register` | Register a new stakeholder |
| `POST` | `/api/v1/auth/login` | Login and receive JWT |
| `POST` | `/api/v1/auth/refresh` | Refresh access token |
| `POST` | `/api/v1/auth/logout` | Invalidate session |

### Battery Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/battery/register` | Register a new battery (Manufacturer only) |
| `GET` | `/api/v1/battery?bpan=<BPAN>` | Retrieve battery by BPAN |

### Health Monitoring

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/battery/health` | Submit health update (SoH, SoC, temperature) |
| `GET` | `/api/v1/battery/health?bpan=<BPAN>` | Get latest health record |

### Carbon Footprint

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/battery/carbon` | Submit carbon footprint data |
| `GET` | `/api/v1/battery/carbon?bpan=<BPAN>` | Get carbon footprint |
| `POST` | `/api/v1/battery/carbon/verify` | Third-party verification |

### Documentation

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/swagger/index.html` | Interactive Swagger UI |
| `GET` | `/healthz` | Liveness probe |

---

## 🧪 Testing

### Run All Tests

```bash
make test
```

### Rust Core Tests

```bash
cd core

# Unit tests
cargo test --lib -- --nocapture --test-threads=4

# Integration tests
cargo test --test integration_test -- --nocapture

# ZK proof tests
cargo test zk_proofs::tests -- --nocapture --test-threads=1

# Audit chain tests
cargo test audit_repo::tests -- --nocapture

# Clippy lint (zero warnings enforced)
cargo clippy --all-targets -- -D warnings

# Security audit
cargo audit
```

### Go API Tests

```bash
cd api

# All tests with race detector
go test -race -cover ./...

# Vet & static analysis
go vet ./...
staticcheck ./...

# BPAN codec tests
go test ./bpan -v
```

### Full Integration Suite

```bash
make test-integration
```

---

## 🐳 Docker Deployment

### Production Stack

```bash
# Start all services
docker compose up -d --build

# View logs
docker compose logs -f

# Stop
docker compose down
```

### Services

| Service | Container | Port | Description |
|---------|-----------|------|-------------|
| PostgreSQL | `bpa-postgres` | 5432 | Primary database |
| Redis | `bpa-redis` | 6379 | Cache & rate limiting |
| Rust Core | `bpa-core` | 50051 | gRPC crypto engine |
| Go API | `bpa-api` | 8080 | REST gateway |

---

## 🏷️ Semantic Versioning

This project follows [Semantic Versioning 2.0.0](https://semver.org/):

```
MAJOR.MINOR.PATCH
  │     │     └─ Bug fixes, security patches (backward compatible)
  │     └─────── New features (backward compatible)
  └───────────── Breaking API changes
```

### Release Process

1. **Tag a release** using the format `vMAJOR.MINOR.PATCH`:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

2. **GitHub Actions** automatically creates a release with:
   - Auto-generated changelog from commit messages
   - Docker image tags matching the semver version
   - Cargo & Go module version validation

### Commit Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/) for automated changelog generation:

| Prefix | Semver Bump | Example |
|--------|-------------|---------|
| `feat:` | MINOR | `feat: add battery health monitoring` |
| `fix:` | PATCH | `fix: resolve clippy range warning` |
| `feat!:` or `BREAKING CHANGE:` | MAJOR | `feat!: redesign BPAN codec format` |
| `docs:` | — | `docs: update README` |
| `ci:` | — | `ci: add cargo audit step` |
| `refactor:` | — | `refactor: box large enum variants` |

### Current Version

| Component | Version |
|-----------|---------|
| `bpa_engine` (Rust) | `0.1.0` |
| `api` (Go) | `v0.1.0` |

---

## 🤝 Contributing

1. **Fork** the repository
2. **Create** a feature branch: `git checkout -b feat/my-feature`
3. **Commit** using Conventional Commits: `git commit -m "feat: add new endpoint"`
4. **Push** to your fork: `git push origin feat/my-feature`
5. **Open a Pull Request** against `develop`

### Code Quality Standards

- ✅ `cargo clippy -- -D warnings` must pass with **zero** warnings
- ✅ `cargo fmt -- --check` must pass
- ✅ `go vet ./...` must pass
- ✅ `staticcheck ./...` must pass
- ✅ All existing tests must pass
- ✅ New features require accompanying tests

---

## 🗺️ Roadmap

- [x] **Phase 1** — Core infrastructure (gRPC, auth, encryption, BPAN codec)
- [x] **Phase 2** — ZK proofs, audit chain, lifecycle tracking, RBAC
- [x] **Phase 3** — Battery registration pipeline, health monitoring, carbon footprint
- [ ] **Phase 4** — Key rotation, lifecycle FSM, second-life marketplace
- [ ] **Phase 5** — EU Battery Regulation compliance, EPREL integration
- [ ] **Phase 6** — Production hardening, observability, horizontal scaling

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).

---

<p align="center">
  Built with 🦀 Rust and 🐹 Go — securing the battery supply chain, one BPAN at a time.
</p>
