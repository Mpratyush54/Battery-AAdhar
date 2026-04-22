# 🔋 Battery Pack Aadhaar (BPA) Core Engine

The BPA Core Engine is a high-performance gRPC service built in Rust that manages the lifecycle, compliance, and regulatory tracking of batteries. It implements the 3-layer data architecture (Static, Dynamic, and Regulatory) as per the BPA guidelines.

## 🚀 Features

- **Battery Registration**: Generates unique 21-character BPANs.
- **Secure Storage**: AES-256-GCM encryption for sensitive battery identifiers.
- **Compliance Tracking**: Automated checks for regulatory violations.
- **Lifecycle Management**: Tracking of ownership, reuse, and recycling.
- **Secrets Management**: Native integration with [Infisical](https://secrets.pratyushes.dev).
- **Audit Logging**: Immutable hash-chained audit logs for all critical actions.

---

## 🛠️ Prerequisites

- **Rust**: [Install Rust](https://rustup.rs/) (Edition 2024).
- **PostgreSQL**: Access to a PostgreSQL instance.
- **Infisical (Optional)**: A self-hosted or cloud Infisical instance for secrets management.

---

## 📦 Installation & Setup

### 1. Clone the repository
```bash
git clone <your-repo-url>
cd Battery/core
```

### 2. Configure Environment Variables
Create a `.env` file in `core/`:

```env
# Database Configuration
DATABASE_URL=postgresql://user:pass@host:port/dbname

# Security
ENCRYPTION_KEY=your-32-character-master-key-here

# Infisical Configuration (Optional - for remote secrets)
INFISICAL_CLIENT_ID=your-client-id
INFISICAL_CLIENT_SECRET=your-client-secret
INFISICAL_PROJECT_ID=your-project-id
INFISICAL_ENV=dev
```

### 3. Build the project
```bash
cargo build
```

---

## 🚦 Running the Engine

On startup, the engine automatically synchronizes the database schema (creates 48 tables if they don't exist).

```bash
cargo run
```

The gRPC server will start on `[::1]:50051`.

---

## 🔐 Secrets Management with Infisical

This project is configured to use a self-hosted Infisical instance at `https://secrets.pratyushes.dev`.

When `INFISICAL_CLIENT_ID` and `INFISICAL_CLIENT_SECRET` are provided, the engine will:
1. Connect to the custom Infisical instance.
2. Fetch `DATABASE_URL` and `ENCRYPTION_KEY` at runtime.
3. Fallback to local `.env` variables if the fetch fails.

---

## 🏗️ Project Structure

- `src/main.rs`: Entry point and gRPC server initialization.
- `src/services/`: Core logic (Registration, Compliance, Ownership, etc.).
- `src/models/`: Database entities and request/response structures.
- `src/errors.rs`: Unified error handling system.
- `proto/`: Protobuf definitions for gRPC.

## 📄 License
MIT License.
