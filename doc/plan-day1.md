# Day 1 — Implementation Plan: Monorepo & Proto Contracts
> Battery Pack Aadhaar · Zero-Knowledge Platform
> Repo: https://github.com/Mpratyush54/Battery-AAdhar

---

## Context snapshot (read before you start)

| Item | Current state |
|------|---------------|
| Repo layout | `api/`, `core/`, `proto/`, root files |
| Rust service | gRPC on `[::1]:50051`, Rust edition 2024 |
| Go workspace | `go.work` + `go.work.sum` present |
| DB schema | `dbschma.txt` — **48 tables** (README claims 43 — R2 must reconcile) |
| Secrets | Infisical self-hosted at `https://secrets.pratyushes.dev` + `.env` fallback |
| Docker | `docker-compose.yaml` exists — **no Redis yet** |

---

## R1 — Rust crypto foundation

### Task 1A — Update `core/Cargo.toml`

Add below the existing `[dependencies]` section.  
**Do not remove existing deps** — only append.

```toml
# ── ZK / Crypto additions (Day 1) ──────────────────────────────────────────
bulletproofs        = { version = "4", default-features = false, features = ["std"] }
curve25519-dalek    = { version = "4", features = ["serde"] }
ed25519-dalek       = { version = "2", features = ["rand_core", "serde"] }
hkdf                = "0.12"
sha2                = "0.10"
rand                = "0.8"
zeroize             = { version = "1", features = ["derive"] }
```

**Why these versions:**
- `bulletproofs = 4` targets `curve25519-dalek = 4`; mismatched majors break compilation.
- `zeroize` is added now — all key material structs will derive `Zeroize` from Day 2 onward.
- `ed25519-dalek = 2` is the current stable (v1 is deprecated).

After editing, run:
```bash
cd core && cargo check 2>&1 | tee /tmp/cargo_check.log
```
Acceptance: exit code 0, `/tmp/cargo_check.log` contains no `error[` lines.

---

### Task 1B — `core/src/services/zk_proofs.rs`

Create this file at exactly that path.

```rust
//! zk_proofs.rs — Zero-knowledge proof service trait
//!
//! All ZK operations are behind this trait so the gRPC handler layer
//! is never coupled to a specific proving system.
//! Concrete implementation (bulletproofs) lands on Day 12.

use std::fmt;

/// Opaque byte blob representing a serialised ZK proof.
/// Consumers must not interpret the bytes — use [`ZkProver::verify`].
#[derive(Debug, Clone, zeroize::Zeroize)]
pub struct ZkProof(pub Vec<u8>);

/// Opaque public inputs for a proof statement.
#[derive(Debug, Clone)]
pub struct ProofPublicInputs(pub Vec<u8>);

/// Error type for ZK operations.
#[derive(Debug)]
pub enum ZkError {
    /// The prover failed to generate a valid proof.
    ProvingFailed(String),
    /// A provided proof did not verify.
    VerificationFailed,
    /// Input value is outside the allowed range.
    OutOfRange { value: u64, min: u64, max: u64 },
    /// Internal error (e.g. RNG failure, serialisation error).
    Internal(String),
}

impl fmt::Display for ZkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZkError::ProvingFailed(msg)        => write!(f, "proving failed: {msg}"),
            ZkError::VerificationFailed        => write!(f, "proof verification failed"),
            ZkError::OutOfRange { value, min, max } =>
                write!(f, "value {value} is outside [{min}, {max}]"),
            ZkError::Internal(msg)             => write!(f, "internal ZK error: {msg}"),
        }
    }
}

impl std::error::Error for ZkError {}

/// The primary interface for all zero-knowledge operations in this service.
///
/// # Stability contract
/// Implementations must be deterministic given the same inputs and randomness
/// source so that proofs can be reproduced for audit purposes.
pub trait ZkProver: Send + Sync {
    /// Prove that `value` lies within `[min, max]` (inclusive) without
    /// revealing `value` to the verifier.
    ///
    /// Used for SoH range proofs:
    /// - `prove_range(soh, 81, 100)` → "battery is operational"
    /// - `prove_range(soh, 60, 80)`  → "battery is second-life eligible"
    fn prove_range(
        &self,
        value: u64,
        min:   u64,
        max:   u64,
    ) -> Result<(ZkProof, ProofPublicInputs), ZkError>;

    /// Verify a range proof produced by [`prove_range`].
    ///
    /// Returns `Ok(())` if the proof is valid, `Err(ZkError::VerificationFailed)`
    /// otherwise. Verifiers receive only the proof and public inputs — never
    /// the raw value.
    fn verify_range(
        &self,
        proof:  &ZkProof,
        public: &ProofPublicInputs,
        min:    u64,
        max:    u64,
    ) -> Result<(), ZkError>;

    /// Prove that a BPAN's static data has not been tampered with since
    /// manufacture, given the manufacturer's signature.
    ///
    /// Returns the proof and the public commitment (hash of signed data).
    fn prove_integrity(
        &self,
        bpan:      &str,
        data_hash: &[u8; 32],
        signature: &[u8],
    ) -> Result<(ZkProof, ProofPublicInputs), ZkError>;

    /// Verify an integrity proof.
    fn verify_integrity(
        &self,
        proof:  &ZkProof,
        public: &ProofPublicInputs,
    ) -> Result<(), ZkError>;
}
```

---

### Task 1C — `core/src/services/signing.rs`

```rust
//! signing.rs — Ed25519 signing and verification service trait
//!
//! Concrete implementation wires in `ed25519-dalek` on Day 13.
//! All key material is wrapped in `Zeroize` types so it is cleared
//! from memory when dropped.

use zeroize::Zeroize;

/// A 64-byte Ed25519 signature.
#[derive(Debug, Clone)]
pub struct Signature(pub [u8; 64]);

/// A 32-byte Ed25519 public key.
#[derive(Debug, Clone)]
pub struct PublicKey(pub [u8; 32]);

/// A 32-byte Ed25519 private key seed — zeroized on drop.
#[derive(Debug, Clone, Zeroize)]
#[zeroize(drop)]
pub struct PrivateKeySeed(pub [u8; 32]);

/// Error type for signing operations.
#[derive(Debug)]
pub enum SigningError {
    /// Key generation or derivation failed.
    KeyError(String),
    /// Signing operation failed.
    SigningFailed(String),
    /// The signature did not verify against the provided public key and message.
    InvalidSignature,
    /// The provided key material is malformed or has an invalid length.
    MalformedKey,
}

impl std::fmt::Display for SigningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SigningError::KeyError(msg)      => write!(f, "key error: {msg}"),
            SigningError::SigningFailed(msg) => write!(f, "signing failed: {msg}"),
            SigningError::InvalidSignature   => write!(f, "signature verification failed"),
            SigningError::MalformedKey       => write!(f, "malformed key material"),
        }
    }
}

impl std::error::Error for SigningError {}

/// The primary interface for Ed25519 signing operations.
///
/// # Key lifecycle
/// - Keys are derived per-manufacturer using HKDF (see `key_manager.rs`).
/// - Public keys are stored in the `certificates` table.
/// - Private key seeds never leave this service boundary.
pub trait SigningService: Send + Sync {
    /// Sign `message` with the manufacturer's private key identified by
    /// `manufacturer_id`. The signing service retrieves the key from the
    /// key manager internally — callers never handle private key material.
    fn sign(
        &self,
        manufacturer_id: &str,
        message:          &[u8],
    ) -> Result<Signature, SigningError>;

    /// Verify `signature` over `message` using `public_key`.
    ///
    /// This is a pure verification — no key lookup required.
    fn verify(
        &self,
        public_key: &PublicKey,
        message:     &[u8],
        signature:   &Signature,
    ) -> Result<(), SigningError>;

    /// Generate a new keypair for a manufacturer.
    /// Returns `(public_key, key_id)` — the private seed is stored internally
    /// and the key_id is what callers use for future `sign()` calls.
    fn generate_keypair(
        &self,
        manufacturer_id: &str,
    ) -> Result<(PublicKey, String), SigningError>;

    /// Retrieve the public key for a given `key_id`.
    fn get_public_key(
        &self,
        key_id: &str,
    ) -> Result<PublicKey, SigningError>;
}
```

### Task 1D — Register new modules in `core/src/services/mod.rs`

Append these lines to the existing `mod.rs` (do not replace existing entries):

```rust
pub mod key_manager;  // HKDF key hierarchy  (stub from R2)
pub mod zk_proofs;    // ZK range proofs      (trait only today)
pub mod signing;      // Ed25519 signing       (trait only today)
```

### R1 acceptance checklist

```bash
cd core
cargo check 2>&1 | grep -c "^error"   # must print 0
cargo check 2>&1 | grep "^warning" | wc -l   # ideally < 10
```

---

## R2 — Schema audit + key manager stub

### Task 2A — Schema audit

Run the following count against the schema file:

```bash
grep -c "^Table " dbschma.txt
```

Expected: 48 (README says 43 — the discrepancy is real and must be documented).

Create `docs/SCHEMA_AUDIT_DAY1.md`:

```markdown
# Schema audit — Day 1
Auditor: R2
Date: [fill in]
Source: dbschma.txt (root of repo)

## Table count
- dbschma.txt tables: 48
- README.md claim: 43
- Discrepancy: 5 extra tables

## Extra tables (not mentioned in README "43 tables" claim)
| Table | Notes |
|-------|-------|
| `kek_keys` | Key-encryption key hierarchy — needed, keep |
| `root_keys` | Root KMS entry — needed, keep |
| `battery_keys` | Per-BPAN DEK store — needed, keep |
| `static_signatures` | Ed25519 sig store — needed, keep |
| `key_destruction_log` | Key EOL audit — needed, keep |

**Recommendation:** Update README to say 48 tables. All 5 extras are load-bearing
for the ZK/encryption architecture. Do NOT remove them.

## Field type audit vs model files

### batteries table
| Schema field | Schema type | Model type | Match? |
|---|---|---|---|
| bpan | varchar | String | ✅ |
| manufacturer_id | uuid | Uuid | ✅ |
| production_year | int | i32 | ✅ |
| battery_category | varchar | String | ✅ |
| compliance_class | varchar | String | ✅ |
| static_hash | varchar | String | ✅ |
| carbon_hash | varchar | String | ✅ |
| created_at | timestamp | DateTime<Utc> | ✅ |

### battery_identifiers table
| Schema field | Schema type | Model type | Match? |
|---|---|---|---|
| id | uuid | Uuid | ✅ |
| bpan | varchar | String | ✅ |
| cipher_algorithm | varchar | String | ✅ |
| cipher_version | int | i32 | ✅ |
| encrypted_serial_number | text | String | ✅ |
| encrypted_batch_number | text | String | ✅ |
| encrypted_factory_code | text | String | ✅ |
| created_at | timestamp | DateTime<Utc> | ✅ |

### battery_keys table (KEY TABLE — verify carefully)
| Schema field | Schema type | Model type | Match? |
|---|---|---|---|
| bpan | varchar (pk) | String | ✅ |
| encrypted_dek | bytea | Vec<u8> | ✅ |
| kek_version | int | i32 | ✅ |
| cipher_algorithm | varchar | String | ✅ |
| cipher_version | int | i32 | ✅ |
| key_status | varchar | String | ✅ |
| created_at | timestamp | DateTime<Utc> | ✅ |
| rotated_at | timestamp | Option<DateTime<Utc>> | ✅ |

### kek_keys table
| Schema field | Schema type | Model type | Match? |
|---|---|---|---|
| id | uuid | Uuid | ✅ |
| encrypted_kek | bytea | Vec<u8> | ✅ |
| version | int | i32 | ✅ |
| root_key_id | uuid | Uuid | ✅ |
| cipher_algorithm | varchar | String | ✅ |
| cipher_version | int | i32 | ✅ |
| status | varchar | String | ✅ |
| created_at | timestamp | DateTime<Utc> | ✅ |
| retired_at | timestamp | Option<DateTime<Utc>> | ✅ |

## Mismatches found
[R2: fill in any actual mismatches you discover — if none, write "0 mismatches found"]

## Action items
- [ ] Update README.md: 43 → 48 tables
- [ ] Confirm all 49 model files in `core/src/models/` have a corresponding table
- [ ] List any model files with NO matching table (orphaned models)
```

### Task 2B — `core/src/services/key_manager.rs`

```rust
//! key_manager.rs — HKDF-based 3-tier key hierarchy stub
//!
//! Key hierarchy (matches dbschma.txt tables):
//!   root_keys  →  kek_keys  →  battery_keys (DEK per BPAN)
//!
//! This is a stub — concrete HKDF derivation wires in on Day 10.
//! All types are defined here so the rest of the codebase can compile
//! against the interface from Day 1.

use zeroize::Zeroize;

/// Key status values — mirror `key_status` / `status` DB columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyStatus {
    Active,
    Retired,
    Destroyed,
}

impl KeyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            KeyStatus::Active    => "active",
            KeyStatus::Retired   => "retired",
            KeyStatus::Destroyed => "destroyed",
        }
    }
}

/// A 32-byte raw key — zeroized on drop.
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct RawKey(pub [u8; 32]);

impl std::fmt::Debug for RawKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print key material
        write!(f, "RawKey([REDACTED])")
    }
}

/// Reference to a KEK version — used when wrapping/unwrapping DEKs.
#[derive(Debug, Clone)]
pub struct KekRef {
    pub id:      uuid::Uuid,
    pub version: i32,
}

/// A wrapped (encrypted) data-encryption key for a single BPAN.
#[derive(Debug, Clone)]
pub struct WrappedDek {
    pub bpan:             String,
    pub encrypted_dek:    Vec<u8>,
    pub kek_version:      i32,
    pub cipher_algorithm: String,
    pub cipher_version:   i32,
}

/// Errors from key management operations.
#[derive(Debug)]
pub enum KeyManagerError {
    /// Root key is not loaded or not hardware-backed.
    RootKeyUnavailable,
    /// KEK for the given version does not exist.
    KekNotFound { version: i32 },
    /// DEK for the given BPAN does not exist.
    DekNotFound { bpan: String },
    /// Key derivation via HKDF failed.
    DerivationFailed(String),
    /// Key wrapping or unwrapping failed.
    WrappingFailed(String),
    /// DB operation failed.
    StorageError(String),
}

impl std::fmt::Display for KeyManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyManagerError::RootKeyUnavailable         => write!(f, "root key unavailable"),
            KeyManagerError::KekNotFound { version }    => write!(f, "KEK version {version} not found"),
            KeyManagerError::DekNotFound { bpan }       => write!(f, "DEK for BPAN {bpan} not found"),
            KeyManagerError::DerivationFailed(msg)      => write!(f, "HKDF derivation failed: {msg}"),
            KeyManagerError::WrappingFailed(msg)        => write!(f, "key wrapping failed: {msg}"),
            KeyManagerError::StorageError(msg)          => write!(f, "storage error: {msg}"),
        }
    }
}

impl std::error::Error for KeyManagerError {}

/// The primary interface for the 3-tier key hierarchy.
///
/// # Tier summary
/// ```
/// Root key (hardware-backed, env or vault)
///   └── KEK  (AES-256-GCM, stored encrypted in kek_keys)
///         └── DEK  (AES-256-GCM, per BPAN, stored in battery_keys)
/// ```
///
/// Callers (encryption service, signing service) only interact with
/// `get_dek_for_bpan` and `create_dek_for_bpan`. The root and KEK
/// tiers are internal to this service.
pub trait KeyManager: Send + Sync {
    /// Derive or retrieve the current KEK.
    /// Only called internally by DEK operations.
    fn get_current_kek(&self) -> Result<(RawKey, KekRef), KeyManagerError>;

    /// Create a new DEK for `bpan`, wrap it with the current KEK,
    /// and persist it to `battery_keys`.
    fn create_dek_for_bpan(
        &self,
        bpan: &str,
    ) -> Result<WrappedDek, KeyManagerError>;

    /// Retrieve and unwrap the DEK for `bpan`.
    /// Returns the plaintext DEK for use in encryption/decryption.
    /// The returned `RawKey` is zeroized when dropped.
    fn get_dek_for_bpan(
        &self,
        bpan: &str,
    ) -> Result<RawKey, KeyManagerError>;

    /// Rotate the DEK for `bpan`: generate new DEK, re-encrypt all
    /// existing encrypted fields, persist new DEK version.
    /// Logs to `key_rotation_log`.
    fn rotate_dek(
        &self,
        bpan:         &str,
        rotated_by:   uuid::Uuid,
    ) -> Result<(), KeyManagerError>;

    /// Destroy the DEK for `bpan` (EOL battery). After this call,
    /// private fields for this BPAN become permanently unreadable.
    /// Logs to `key_destruction_log`.
    fn destroy_dek(
        &self,
        bpan:              &str,
        destroyed_by:      uuid::Uuid,
        destruction_method: &str,
    ) -> Result<(), KeyManagerError>;
}
```

### R2 acceptance checklist

```bash
# 1. Audit doc exists
ls docs/SCHEMA_AUDIT_DAY1.md

# 2. Rust compiles with new module
cd core && cargo check 2>&1 | grep -c "^error"   # must be 0

# 3. Schema table count
grep -c "^Table " dbschma.txt   # must be 48
```

---

## G1 — Go chi router refactor

### Task 3A — Add chi to `api/go.mod`

```bash
cd api
go get github.com/go-chi/chi/v5@latest
go get github.com/go-chi/chi/v5/middleware@latest
go mod tidy
```

Commit the resulting `go.mod` and `go.sum`.

### Task 3B — Refactor `api/routes/routes.go`

Replace the file entirely:

```go
// routes.go — chi-based router for the Battery Aadhaar API
// Replaces the previous http.ServeMux implementation.
// All existing route paths are preserved; only the router type changes.
package routes

import (
	"net/http"

	"github.com/go-chi/chi/v5"
	chiMiddleware "github.com/go-chi/chi/v5/middleware"

	"github.com/Mpratyush54/Battery-AAdhar/api/middleware"
	// Import your existing handlers — adjust import paths as needed
)

// NewRouter constructs and returns the application chi.Router.
// All middleware is applied here in the correct order:
//   1. chi built-ins (request ID, real IP, recoverer)
//   2. custom logging   (structured zap/slog output)
//   3. custom auth      (JWT parse + attach claims to context)
//   4. custom RBAC      (role enforcement per route group)
func NewRouter() http.Handler {
	r := chi.NewRouter()

	// ── Global middleware (runs on every request) ─────────────────────────
	r.Use(chiMiddleware.RequestID)
	r.Use(chiMiddleware.RealIP)
	r.Use(chiMiddleware.Recoverer)
	r.Use(middleware.Logger)      // structured logging stub
	r.Use(middleware.Authenticate) // JWT parse — does NOT reject; just attaches claims

	// ── Health / readiness ────────────────────────────────────────────────
	r.Get("/healthz", handleHealthz)
	r.Get("/readyz",  handleReadyz)

	// ── API v1 ────────────────────────────────────────────────────────────
	r.Route("/api/v1", func(r chi.Router) {

		// Public endpoints — no auth required beyond claim parse
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireRole("public"))
			r.Get("/batteries/{bpan}", handleGetBattery)
			r.Post("/batteries/scan",  handleScanQR)
		})

		// Authenticated manufacturer endpoints
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireRole("manufacturer"))
			r.Post("/batteries",              handleRegisterBattery)
			r.Get("/batteries/{bpan}/qr",     handleGetQR)
		})

		// Service provider / recycler endpoints
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireRole("service_provider"))
			r.Get("/batteries/{bpan}/private",          handleGetPrivateData)
			r.Patch("/batteries/{bpan}/status",         handleUpdateStatus)
		})

		// Compliance / ZK verification endpoints
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireRole("verifier"))
			r.Post("/batteries/{bpan}/verify/operational", handleVerifyOperational)
			r.Post("/batteries/{bpan}/verify/recyclable",  handleVerifyRecyclable)
			r.Post("/batteries/{bpan}/verify/signature",   handleVerifySignature)
		})

		// Admin-only
		r.Group(func(r chi.Router) {
			r.Use(middleware.RequireRole("admin"))
			r.Post("/manufacturers",       handleRegisterManufacturer)
			r.Get("/manufacturers",        handleListManufacturers)
		})
	})

	return r
}

// ── Placeholder handlers (replace with real handlers as they are built) ──

func handleHealthz(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write([]byte(`{"status":"ok"}`))
}

func handleReadyz(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write([]byte(`{"status":"ready"}`))
}

func handleGetBattery(w http.ResponseWriter, _ *http.Request)          { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleScanQR(w http.ResponseWriter, _ *http.Request)              { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleRegisterBattery(w http.ResponseWriter, _ *http.Request)     { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleGetQR(w http.ResponseWriter, _ *http.Request)               { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleGetPrivateData(w http.ResponseWriter, _ *http.Request)      { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleUpdateStatus(w http.ResponseWriter, _ *http.Request)        { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleVerifyOperational(w http.ResponseWriter, _ *http.Request)   { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleVerifyRecyclable(w http.ResponseWriter, _ *http.Request)    { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleVerifySignature(w http.ResponseWriter, _ *http.Request)     { http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleRegisterManufacturer(w http.ResponseWriter, _ *http.Request){ http.Error(w, "not implemented", http.StatusNotImplemented) }
func handleListManufacturers(w http.ResponseWriter, _ *http.Request)   { http.Error(w, "not implemented", http.StatusNotImplemented) }
```

### Task 3C — `api/middleware/logging.go`

```go
// logging.go — structured request logging middleware
package middleware

import (
	"log/slog"
	"net/http"
	"time"

	"github.com/go-chi/chi/v5/middleware"
)

// Logger is a chi-compatible middleware that emits a structured slog record
// for every request. Replace slog with zap in a follow-up PR if needed.
func Logger(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()
		ww    := middleware.NewWrapResponseWriter(w, r.ProtoMajor)

		defer func() {
			slog.Info("request",
				"method",     r.Method,
				"path",       r.URL.Path,
				"status",     ww.Status(),
				"bytes",      ww.BytesWritten(),
				"duration_ms", time.Since(start).Milliseconds(),
				"request_id", middleware.GetReqID(r.Context()),
				"remote_ip",  r.RemoteAddr,
			)
		}()

		next.ServeHTTP(ww, r)
	})
}
```

### Task 3D — `api/middleware/auth.go`

```go
// auth.go — JWT parsing middleware (does NOT enforce; just attaches claims)
// Role enforcement is in rbac.go — RequireRole().
package middleware

import (
	"context"
	"net/http"
	"strings"
)

// contextKey is unexported to avoid collisions with other packages.
type contextKey string

const (
	claimsKey contextKey = "claims"
)

// Claims holds the parsed JWT payload for a request.
// Full JWT validation (RS256 signature, expiry) is added on Day 15.
type Claims struct {
	Subject        string   `json:"sub"`
	Role           string   `json:"role"`
	ManufacturerID string   `json:"manufacturer_id,omitempty"`
	Permissions    []string `json:"permissions,omitempty"`
}

// Authenticate parses the Authorization: Bearer <token> header and attaches
// Claims to the request context. Requests without a token get a guest/public
// Claims so downstream handlers always find a non-nil value.
//
// STUB: On Day 15 this will verify the RS256 signature and expiry.
// Today it only parses the header so the middleware chain compiles.
func Authenticate(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		claims := &Claims{Role: "public"} // default: unauthenticated guest

		authHeader := r.Header.Get("Authorization")
		if strings.HasPrefix(authHeader, "Bearer ") {
			token := strings.TrimPrefix(authHeader, "Bearer ")
			if token != "" {
				// TODO Day 15: verify RS256, parse sub/role from JWT
				// For now, just mark as authenticated with placeholder
				claims = &Claims{
					Subject: "stub-subject",
					Role:    "authenticated",
				}
			}
		}

		ctx := context.WithValue(r.Context(), claimsKey, claims)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}

// ClaimsFromContext retrieves the Claims attached by Authenticate.
// Returns nil if called before Authenticate runs (should not happen).
func ClaimsFromContext(ctx context.Context) *Claims {
	v, _ := ctx.Value(claimsKey).(*Claims)
	return v
}
```

### Task 3E — `api/middleware/rbac.go`

```go
// rbac.go — role-based access control middleware
// Role hierarchy (lowest → highest privilege):
//   public < authenticated < manufacturer < service_provider < recycler < verifier < government < admin
package middleware

import (
	"net/http"
	"slices"
)

// roleHierarchy defines the ordered privilege tiers.
// A role grants access to its own tier and all tiers below it.
var roleHierarchy = []string{
	"public",
	"authenticated",
	"manufacturer",
	"service_provider",
	"recycler",
	"verifier",
	"government",
	"admin",
}

func roleLevel(role string) int {
	idx := slices.Index(roleHierarchy, role)
	if idx < 0 {
		return -1 // unknown role → no access
	}
	return idx
}

// RequireRole returns a middleware that rejects requests whose JWT role
// is below the required level.
func RequireRole(required string) func(http.Handler) http.Handler {
	requiredLevel := roleLevel(required)

	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			claims := ClaimsFromContext(r.Context())
			if claims == nil || roleLevel(claims.Role) < requiredLevel {
				http.Error(w,
					`{"error":"insufficient_role","required":"`+required+`"}`,
					http.StatusForbidden,
				)
				return
			}
			next.ServeHTTP(w, r)
		})
	}
}
```

### G1 acceptance checklist

```bash
cd api
go build ./...          # must exit 0
go test ./...           # must exit 0 (all existing tests still pass)

# Verify chi is in go.mod
grep "go-chi/chi" go.mod   # must find the line

# Spot check: /healthz returns 200
# (Only works if you run the server: go run main.go)
```

---

## G2 — buf setup & proto split

### Task 4A — Install buf

```bash
# macOS
brew install bufbuild/buf/buf

# Linux
curl -sSL https://github.com/bufbuild/buf/releases/latest/download/buf-Linux-x86_64 \
  -o /usr/local/bin/buf && chmod +x /usr/local/bin/buf

buf --version   # confirm installed
```

### Task 4B — `buf.yaml` (repo root)

```yaml
version: v2
name: buf.build/battery-aadhaar/api
lint:
  use:
    - DEFAULT
  except:
    - PACKAGE_DIRECTORY_MATCH   # we use flat proto/ dir for now
breaking:
  use:
    - FILE
```

### Task 4C — `buf.gen.yaml` (repo root)

```yaml
version: v2
plugins:
  # Go stubs (for api/ service)
  - plugin: buf.build/protocolbuffers/go
    out: api/gen/proto
    opt:
      - paths=source_relative

  # Go gRPC service stubs
  - plugin: buf.build/grpc/go
    out: api/gen/proto
    opt:
      - paths=source_relative

  # Rust stubs (for core/ service via tonic-build in build.rs)
  # tonic-build generates from proto at compile time — no buf plugin needed
  # but we keep buf lint authoritative for all .proto files
```

Create the output directory:
```bash
mkdir -p api/gen/proto
echo "*.pb.go" >> api/gen/proto/.gitignore
echo "*_grpc.pb.go" >> api/gen/proto/.gitignore
```

### Task 4D — Proto file split

Split the existing `proto/bpa.proto` into 5 focused files.
Create each file below:

---

**`proto/common.proto`** — shared types used by all services

```protobuf
syntax = "proto3";

package bpa.common.v1;

option go_package = "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/common/v1;commonv1";

import "google/protobuf/timestamp.proto";

// ─── BPAN ──────────────────────────────────────────────────────────────────

// A validated 21-character Battery Pack Aadhaar Number.
message Bpan {
  string value = 1; // e.g. "MY008A6FKKKLC1DH80001"
}

// ─── Pagination ────────────────────────────────────────────────────────────

message PageRequest {
  int32  page_size  = 1;
  string page_token = 2;
}

message PageResponse {
  string next_page_token = 1;
  int32  total_count     = 2;
}

// ─── Common error details ──────────────────────────────────────────────────

message FieldViolation {
  string field   = 1;
  string message = 2;
}

// ─── Timestamp alias ───────────────────────────────────────────────────────

// Re-exported for consistent import across services.
// Use google.protobuf.Timestamp directly in message definitions.
```

---

**`proto/crypto.proto`** — Rust crypto service RPC contracts

```protobuf
syntax = "proto3";

package bpa.crypto.v1;

option go_package = "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/crypto/v1;cryptov1";

import "google/protobuf/timestamp.proto";

// ─── Encryption ────────────────────────────────────────────────────────────

message EncryptRequest {
  string bpan            = 1; // used as AAD for AES-GCM
  string field_name      = 2; // e.g. "cathode_material"
  bytes  plaintext       = 3;
  int32  kek_version     = 4; // 0 = use current
}

message EncryptResponse {
  bytes  ciphertext        = 1; // AES-256-GCM ciphertext + nonce + tag
  int32  kek_version_used  = 2;
  int32  cipher_version    = 3;
  string cipher_algorithm  = 4; // "AES-256-GCM"
}

message DecryptRequest {
  string bpan            = 1;
  string field_name      = 2;
  bytes  ciphertext      = 3;
  int32  kek_version     = 4;
  string cipher_algorithm= 5;
}

message DecryptResponse {
  bytes plaintext = 1;
}

// ─── Signing ───────────────────────────────────────────────────────────────

message SignRequest {
  string manufacturer_id = 1;
  bytes  message         = 2; // raw bytes to sign
}

message SignResponse {
  bytes  signature = 1; // 64-byte Ed25519 signature
  string key_id    = 2;
}

message VerifyRequest {
  bytes  public_key = 1; // 32-byte Ed25519 public key
  bytes  message    = 2;
  bytes  signature  = 3;
}

message VerifyResponse {
  bool valid = 1;
}

// ─── ZK Proofs ─────────────────────────────────────────────────────────────

enum RangeProofType {
  RANGE_PROOF_TYPE_UNSPECIFIED  = 0;
  RANGE_PROOF_TYPE_OPERATIONAL  = 1; // SoH > 80
  RANGE_PROOF_TYPE_SECOND_LIFE  = 2; // 60 <= SoH <= 80
  RANGE_PROOF_TYPE_RECYCLABLE   = 3; // recyclability% >= threshold
}

message ZkProveRequest {
  RangeProofType proof_type  = 1;
  uint64         value       = 2; // the secret value being proved
  uint64         range_min   = 3;
  uint64         range_max   = 4;
}

message ZkProveResponse {
  bytes proof         = 1; // serialised bulletproof
  bytes public_inputs = 2;
}

message ZkVerifyRequest {
  bytes          proof         = 1;
  bytes          public_inputs = 2;
  RangeProofType proof_type    = 3;
  uint64         range_min     = 4;
  uint64         range_max     = 5;
}

message ZkVerifyResponse {
  bool valid = 1;
}

// ─── Key management RPCs ───────────────────────────────────────────────────

message GenerateKeyPairRequest {
  string manufacturer_id = 1;
}

message GenerateKeyPairResponse {
  bytes  public_key = 1;
  string key_id     = 2;
}

message RotateDekRequest {
  string bpan           = 1;
  string rotated_by_id  = 2;
}

message RotateDekResponse {
  bool   success    = 1;
  int32  new_version = 2;
}

// ─── Service definition ────────────────────────────────────────────────────

service CryptoService {
  // Encryption
  rpc Encrypt (EncryptRequest)  returns (EncryptResponse);
  rpc Decrypt (DecryptRequest)  returns (DecryptResponse);

  // Signing
  rpc Sign              (SignRequest)             returns (SignResponse);
  rpc Verify            (VerifyRequest)           returns (VerifyResponse);
  rpc GenerateKeyPair   (GenerateKeyPairRequest)  returns (GenerateKeyPairResponse);

  // ZK proofs
  rpc ZkProve  (ZkProveRequest)  returns (ZkProveResponse);
  rpc ZkVerify (ZkVerifyRequest) returns (ZkVerifyResponse);

  // Key lifecycle
  rpc RotateDek (RotateDekRequest) returns (RotateDekResponse);
}
```

---

**`proto/battery.proto`** — battery registration + data RPCs

```protobuf
syntax = "proto3";

package bpa.battery.v1;

option go_package = "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/battery/v1;batteryv1";

import "google/protobuf/timestamp.proto";
import "proto/common.proto";

// ─── Enums ─────────────────────────────────────────────────────────────────

enum BatteryCategory {
  BATTERY_CATEGORY_UNSPECIFIED = 0;
  EV_L_CATEGORY                = 1;
  EV_M_N_CATEGORY              = 2;
  INDUSTRIAL_ABOVE_2KWH        = 3;
}

enum BatteryStatus {
  BATTERY_STATUS_UNSPECIFIED = 0;
  OPERATIONAL                = 1; // SoH > 80%
  SECOND_LIFE                = 2; // 60–80%
  END_OF_LIFE                = 3; // < 60%
  WASTE                      = 4;
}

// ─── Static data (uploaded once at manufacture) ────────────────────────────

message BatteryStaticData {
  // BMI
  string country_code       = 1;
  string manufacturer_code  = 2;

  // BDS
  float  battery_capacity_kwh = 3;
  string battery_chemistry    = 4;
  float  nominal_voltage      = 5;
  string cell_origin          = 6;
  string extinguisher_class   = 7;

  // BI
  string manufacturing_date   = 8; // YYYYMMDD
  string factory_code         = 9;
  string sequential_number    = 10;

  // BMCS (public subset)
  string tac_number              = 11;
  int32  num_cells               = 12;
  float  internal_resistance_ohm = 13;
  float  weight_kg               = 14;
  int32  warranty_years          = 15;
  string cell_type               = 16;

  // BCF (public: total only)
  float  total_carbon_footprint_kgco2e_per_kwh = 17;
}

// ─── Dynamic data (BDD) ────────────────────────────────────────────────────

message BatteryDynamicData {
  string         bpan              = 1;
  BatteryCategory category         = 2;
  BatteryStatus   status           = 3;
  float           state_of_health  = 4; // 0.0–100.0
  string          disassembly_method = 5;
  string          circularity_method = 6;
  google.protobuf.Timestamp updated_at = 7;
}

// ─── Requests / Responses ──────────────────────────────────────────────────

message RegisterBatteryRequest {
  string            manufacturer_id = 1;
  BatteryStaticData static_data     = 2;
}

message RegisterBatteryResponse {
  string bpan          = 1; // generated 21-char BPAN
  bytes  qr_code_png   = 2; // QR code image
  string qr_payload    = 3; // raw QR string for offline decode
}

message GetBatteryRequest {
  string bpan = 1;
}

message GetBatteryResponse {
  string            bpan        = 1;
  BatteryStaticData static_data = 2;
  // private fields are NOT included — use GetPrivateBatteryData
}

message UpdateBatteryStatusRequest {
  string  bpan              = 1;
  float   state_of_health   = 2;
  BatteryStatus new_status  = 3;
  string  updated_by_id     = 4;
  string  disassembly_method = 5;
}

message UpdateBatteryStatusResponse {
  bool   success    = 1;
  string new_status = 2;
}

// ─── Service ───────────────────────────────────────────────────────────────

service BatteryService {
  rpc RegisterBattery    (RegisterBatteryRequest)     returns (RegisterBatteryResponse);
  rpc GetBattery         (GetBatteryRequest)          returns (GetBatteryResponse);
  rpc UpdateBatteryStatus(UpdateBatteryStatusRequest) returns (UpdateBatteryStatusResponse);
}
```

---

**`proto/auth.proto`** — authentication + stakeholder management

```protobuf
syntax = "proto3";

package bpa.auth.v1;

option go_package = "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/auth/v1;authv1";

import "google/protobuf/timestamp.proto";

enum StakeholderRole {
  STAKEHOLDER_ROLE_UNSPECIFIED = 0;
  PUBLIC                       = 1;
  MANUFACTURER                 = 2;
  IMPORTER                     = 3;
  DISTRIBUTOR                  = 4;
  SERVICE_PROVIDER             = 5;
  RECYCLER                     = 6;
  GOVERNMENT                   = 7;
  ADMIN                        = 8;
}

message IssueTokenRequest {
  string client_id     = 1;
  string client_secret = 2;
}

message IssueTokenResponse {
  string access_token  = 1;
  int64  expires_in    = 2;
  string token_type    = 3; // "Bearer"
}

message CheckRoleRequest {
  string token    = 1;
  string resource = 2;
  string action   = 3;
}

message CheckRoleResponse {
  bool   allowed = 1;
  string reason  = 2;
}

message RegisterManufacturerRequest {
  string name         = 1;
  string country_code = 2;
}

message RegisterManufacturerResponse {
  string manufacturer_id   = 1;
  string assigned_bmi      = 2; // e.g. "MY008"
  string api_client_id     = 3;
  string api_client_secret = 4; // returned ONCE — store securely
}

service AuthService {
  rpc IssueToken           (IssueTokenRequest)           returns (IssueTokenResponse);
  rpc CheckRole            (CheckRoleRequest)            returns (CheckRoleResponse);
  rpc RegisterManufacturer (RegisterManufacturerRequest) returns (RegisterManufacturerResponse);
}
```

---

**`proto/lifecycle.proto`** — ZK compliance + lifecycle verification

```protobuf
syntax = "proto3";

package bpa.lifecycle.v1;

option go_package = "github.com/Mpratyush54/Battery-AAdhar/api/gen/proto/lifecycle/v1;lifecyclev1";

import "google/protobuf/timestamp.proto";

message VerifyOperationalRequest {
  string bpan         = 1;
  string requester_id = 2;
}

message VerifyOperationalResponse {
  bool   is_operational = 1;
  bytes  zk_proof       = 2; // bulletproof bytes — verifiable offline
  bytes  public_inputs  = 3;
  google.protobuf.Timestamp proof_issued_at = 4;
  int64  proof_valid_until_unix = 5;
}

message VerifyRecyclableRequest {
  string bpan                      = 1;
  float  min_recyclability_percent = 2;
}

message VerifyRecyclableResponse {
  bool   meets_threshold = 1;
  bytes  zk_proof        = 2;
  bytes  public_inputs   = 3;
}

message VerifySignatureRequest {
  string bpan = 1;
}

message VerifySignatureResponse {
  bool   tamper_evident     = 1;
  string signer_key_id      = 2;
  google.protobuf.Timestamp signed_at = 3;
}

service LifecycleService {
  rpc VerifyOperational (VerifyOperationalRequest) returns (VerifyOperationalResponse);
  rpc VerifyRecyclable  (VerifyRecyclableRequest)  returns (VerifyRecyclableResponse);
  rpc VerifySignature   (VerifySignatureRequest)   returns (VerifySignatureResponse);
}
```

### G2 acceptance checklist

```bash
# 1. buf lint
buf lint proto/   # must exit 0 with no output

# 2. buf generate (Go stubs only — Rust uses tonic-build)
buf generate      # must exit 0

# 3. Generated files exist
ls api/gen/proto/   # must contain *.pb.go files

# 4. No breaking changes introduced vs original bpa.proto
buf breaking --against ".git#branch=master" proto/
```

---

## D1 — Makefile, CI, and docker-compose Redis

### Task 5A — `Makefile` (repo root)

```makefile
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
```

### Task 5B — `.github/workflows/ci.yml`

```yaml
# ci.yml — Battery Aadhaar CI pipeline
# Triggers: every PR to master, every push to master

name: CI

on:
  push:
    branches: [master]
  pull_request:
    branches: [master]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  # ── Proto lint ────────────────────────────────────────────────────────────
  proto-lint:
    name: Proto lint (buf)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: bufbuild/buf-setup-action@v1
        with:
          version: latest

      - name: Lint proto files
        run: buf lint proto/

      - name: Check breaking changes
        run: buf breaking --against '.git#branch=master' proto/
        # On the first run against master this will fail if base is empty;
        # wrap in || true until there is a stable baseline
        continue-on-error: true

  # ── Rust ──────────────────────────────────────────────────────────────────
  rust:
    name: Rust — check, clippy, test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain (stable)
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            core/target
          key: ${{ runner.os }}-cargo-${{ hashFiles('core/Cargo.lock') }}
          restore-keys: ${{ runner.os }}-cargo-

      - name: cargo check
        working-directory: core
        run: cargo check

      - name: cargo clippy
        working-directory: core
        run: cargo clippy -- -D warnings

      - name: cargo test
        working-directory: core
        run: cargo test -- --test-threads=4

  # ── Go ────────────────────────────────────────────────────────────────────
  go:
    name: Go — build, vet, test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Go
        uses: actions/setup-go@v5
        with:
          go-version-file: api/go.mod
          cache-dependency-path: api/go.sum

      - name: go build
        working-directory: api
        run: go build ./...

      - name: go vet
        working-directory: api
        run: go vet ./...

      - name: go test
        working-directory: api
        run: go test -race -coverprofile=coverage.out ./...

      - name: Upload coverage
        uses: actions/upload-artifact@v4
        with:
          name: go-coverage
          path: api/coverage.out

  # ── Docker build smoke test ───────────────────────────────────────────────
  docker-smoke:
    name: Docker build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build images
        run: docker compose build --no-cache

      - name: Verify containers start
        run: |
          docker compose up -d
          sleep 10
          docker compose ps
          docker compose down
```

### Task 5C — Updated `docker-compose.yaml` (add Redis)

Add the Redis service to the existing file. Find the `services:` block and append:

```yaml
  # ── Redis (cache + ZK proof cache + rate limiting) ───────────────────────
  redis:
    image: redis:7-alpine
    container_name: bpa-redis
    restart: unless-stopped
    ports:
      - "6379:6379"
    command: >
      redis-server
      --maxmemory 256mb
      --maxmemory-policy allkeys-lru
      --save ""
      --appendonly no
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 3s
      retries: 5
    networks:
      - bpa-network
```

If `networks:` does not exist in the file yet, add this at the root level:

```yaml
networks:
  bpa-network:
    driver: bridge
```

Add the `bpa-network` entry to the `networks:` list of your existing `postgres` / `core` / `api` services as well so all containers can resolve each other by name.

Also add the `REDIS_URL` environment variable to the `api` service:
```yaml
    environment:
      REDIS_URL: redis://redis:6379
```

### D1 acceptance checklist

```bash
# 1. Makefile targets work
make proto          # buf lint + generate — exit 0
make build-rust     # cargo build --release — exit 0
make build-go       # go build ./...       — exit 0
make test           # both test suites     — exit 0

# 2. Docker stack comes up with Redis
make docker-up
docker compose ps   # all services "Up", health "healthy"
docker exec bpa-redis redis-cli ping   # must return PONG

# 3. CI file is valid YAML
python -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"

# 4. Git push triggers CI
# Open a PR → Actions tab → all 4 jobs must appear
```

---

## End-of-day verification (team lead)

Run this script from repo root after all 5 people have merged their work:

```bash
#!/usr/bin/env bash
set -e

echo "=== Day 1 acceptance gate ==="

# R1: Rust compiles
echo "→ R1: cargo check..."
(cd core && cargo check 2>&1 | grep -c "^error") | grep -q "^0$" && echo "✓ R1 pass" || echo "✗ R1 FAIL"

# R2: Audit doc exists + Rust still compiles
echo "→ R2: schema audit doc..."
test -f docs/SCHEMA_AUDIT_DAY1.md && echo "✓ R2 audit doc" || echo "✗ R2 FAIL: missing audit doc"

# G1: Go builds and tests pass
echo "→ G1: go build + test..."
(cd api && go build ./... && go test ./...) && echo "✓ G1 pass" || echo "✗ G1 FAIL"

# G2: buf lint passes
echo "→ G2: buf lint..."
buf lint proto/ && echo "✓ G2 pass" || echo "✗ G2 FAIL"

# D1: Docker with Redis
echo "→ D1: docker compose up..."
docker compose up -d --build
sleep 15
docker exec bpa-redis redis-cli ping | grep -q "PONG" && echo "✓ D1 Redis up" || echo "✗ D1 FAIL"
docker compose down

echo "=== Done ==="
```

---

## Dependency + version pinning reference

| Library | Ecosystem | Version | Purpose |
|---------|-----------|---------|---------|
| `bulletproofs` | Rust | 4.x | ZK range proofs |
| `curve25519-dalek` | Rust | 4.x | Elliptic curve primitives |
| `ed25519-dalek` | Rust | 2.x | Ed25519 signing |
| `hkdf` | Rust | 0.12 | HKDF key derivation |
| `sha2` | Rust | 0.10 | SHA-256 for HKDF + hashing |
| `zeroize` | Rust | 1.x | Secure memory zeroing |
| `go-chi/chi/v5` | Go | latest | HTTP router |
| `buf` | CLI | latest | Proto lint + codegen |
| `redis 7-alpine` | Docker | 7 | Cache + rate-limit store |

---

*Day 1 target: green CI pipeline, all 4 jobs passing, Docker stack (Postgres + Redis + Rust + Go) healthy.*