# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-05

### Added
- **Core Engine (Rust)**
  - gRPC server with Protobuf v3 service definitions
  - 3-tier Key Management System (Root → KEK → DEK) with AES-256-GCM
  - Bulletproofs-based Zero-Knowledge SoH verification
  - Append-only audit chain with SHA-256 hash linking
  - Battery registration pipeline with atomic BPAN issuance
  - Health monitoring service (SoH, SoC, temperature tracking)
  - Carbon footprint 5-stage emissions model (BCF)
  - Lifecycle state machine (Manufacturing → Operational → SecondLife → Recycling)
  - mTLS support for gRPC endpoints

- **API Gateway (Go)**
  - REST API with Swagger/OpenAPI auto-documentation
  - JWT authentication with bcrypt password hashing
  - Role-Based Access Control (Manufacturer, Regulator, Recycler, End-user)
  - BPAN codec (21-char encode/decode)
  - gRPC client factory with automatic mTLS/insecure detection
  - Structured logging middleware with request tracing
  - Health, carbon, and battery controllers

- **Infrastructure**
  - Docker Compose stack (Postgres 15, Redis 7, Rust Core, Go API)
  - GitHub Actions CI pipeline (proto lint, clippy, test, audit, vet, staticcheck)
  - Makefile with build/test/lint/docker targets
  - mTLS certificate generation scripts
  - Infisical secret manager integration
  - Degraded mode for CI environments without live databases

### Security
- Enforced `cargo clippy -- -D warnings` zero-warning policy
- Boxed large enum variants to prevent stack overflow
- Idiomatic Rust range checks via `.contains()`
- Proper `Display` trait implementations (no inherent `to_string()`)
