#!/usr/bin/env bash
# generate-certs.sh — Generate mTLS certificates for Rust ↔ Go communication
#
# Creates:
#   - CA certificate (self-signed root)
#   - Server certificate (Rust gRPC service)
#   - Client certificate (Go client)
#
# Usage: ./scripts/generate-certs.sh

set -e

CERTS_DIR="certs"
VALIDITY_DAYS=365
COUNTRY="IN"
STATE="TN"
CITY="Chennai"
ORG="BPA"

echo "🔐 Generating mTLS certificates..."

# Create certs directory
mkdir -p "$CERTS_DIR"

# ── Step 1: Generate CA private key ───────────────────────────────────────
echo "→ Generating CA private key..."
openssl genrsa -out "$CERTS_DIR/ca.key" 4096 2>/dev/null

# ── Step 2: Generate CA certificate ──────────────────────────────────────
echo "→ Generating CA certificate (self-signed)..."
openssl req -new -x509 -days $VALIDITY_DAYS -key "$CERTS_DIR/ca.key" \
  -subj "/C=$COUNTRY/ST=$STATE/L=$CITY/O=$ORG/CN=BPA-CA" \
  -out "$CERTS_DIR/ca.crt" 2>/dev/null

# ── Step 3: Generate Server (Rust) private key ───────────────────────────
echo "→ Generating server private key..."
openssl genrsa -out "$CERTS_DIR/server.key" 4096 2>/dev/null

# ── Step 4: Generate Server CSR ──────────────────────────────────────────
echo "→ Generating server CSR..."
openssl req -new -key "$CERTS_DIR/server.key" \
  -subj "/C=$COUNTRY/ST=$STATE/L=$CITY/O=$ORG/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,DNS:core,IP:127.0.0.1" \
  -out "$CERTS_DIR/server.csr" 2>/dev/null

# ── Step 5: Sign Server CSR with CA ──────────────────────────────────────
echo "→ Signing server certificate with CA..."
openssl x509 -req -days $VALIDITY_DAYS \
  -in "$CERTS_DIR/server.csr" \
  -CA "$CERTS_DIR/ca.crt" -CAkey "$CERTS_DIR/ca.key" \
  -CAcreateserial -out "$CERTS_DIR/server.crt" \
  -extfile <(printf "subjectAltName=DNS:localhost,DNS:core,IP:127.0.0.1") \
  2>/dev/null

# ── Step 6: Generate Client (Go) private key ─────────────────────────────
echo "→ Generating client private key..."
openssl genrsa -out "$CERTS_DIR/client.key" 4096 2>/dev/null

# ── Step 7: Generate Client CSR ──────────────────────────────────────────
echo "→ Generating client CSR..."
openssl req -new -key "$CERTS_DIR/client.key" \
  -subj "/C=$COUNTRY/ST=$STATE/L=$CITY/O=$ORG/CN=bpa-client" \
  -out "$CERTS_DIR/client.csr" 2>/dev/null

# ── Step 8: Sign Client CSR with CA ──────────────────────────────────────
echo "→ Signing client certificate with CA..."
openssl x509 -req -days $VALIDITY_DAYS \
  -in "$CERTS_DIR/client.csr" \
  -CA "$CERTS_DIR/ca.crt" -CAkey "$CERTS_DIR/ca.key" \
  -CAcreateserial -out "$CERTS_DIR/client.crt" \
  2>/dev/null

# ── Step 9: Cleanup CSR files ────────────────────────────────────────────
rm -f "$CERTS_DIR"/*.csr "$CERTS_DIR"/ca.srl

# ── Step 10: Verify certificates ────────────────────────────────────────
echo "✓ Certificate generation complete!"
echo ""
echo "📋 Certificate details:"
echo ""
echo "CA Certificate:"
openssl x509 -in "$CERTS_DIR/ca.crt" -noout -text | grep -E "Subject:|Issuer:|Not Before|Not After" | head -4

echo ""
echo "Server Certificate:"
openssl x509 -in "$CERTS_DIR/server.crt" -noout -text | grep -E "Subject:|CN=" | head -1

echo ""
echo "Client Certificate:"
openssl x509 -in "$CERTS_DIR/client.crt" -noout -text | grep -E "Subject:|CN=" | head -1

echo ""
echo "📁 Generated files:"
ls -lh "$CERTS_DIR"/

# ── Verify CA signed the certificates ────────────────────────────────────
echo ""
echo "🔍 Verifying certificates..."
if openssl verify -CAfile "$CERTS_DIR/ca.crt" "$CERTS_DIR/server.crt" > /dev/null 2>&1; then
  echo "✓ Server certificate: VALID"
else
  echo "✗ Server certificate: INVALID"
  exit 1
fi

if openssl verify -CAfile "$CERTS_DIR/ca.crt" "$CERTS_DIR/client.crt" > /dev/null 2>&1; then
  echo "✓ Client certificate: VALID"
else
  echo "✗ Client certificate: INVALID"
  exit 1
fi

echo ""
echo "✅ mTLS setup complete!"
echo ""
echo "Next steps:"
echo "  1. Rust server uses: certs/server.crt, certs/server.key, certs/ca.crt"
echo "  2. Go client uses: certs/client.crt, certs/client.key, certs/ca.crt"
echo "  3. Set environment variables:"
echo "     export GRPC_SERVER_CERT=certs/server.crt"
echo "     export GRPC_SERVER_KEY=certs/server.key"
echo "     export GRPC_CA_CERT=certs/ca.crt"
