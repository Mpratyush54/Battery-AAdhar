
$CERTS_DIR = "certs"
$VALIDITY_DAYS = 365
$COUNTRY = "IN"
$STATE = "TN"
$CITY = "Chennai"
$ORG = "BPA"

Write-Host "Generating mTLS certificates..."

# Create certs directory
if (-not (Test-Path -Path $CERTS_DIR)) {
    New-Item -ItemType Directory -Path $CERTS_DIR | Out-Null
}

# Step 1: Generate CA private key
Write-Host "Generating CA private key..."
openssl genrsa -out "$CERTS_DIR/ca.key" 4096 2>$null

# Step 2: Generate CA certificate
Write-Host "Generating CA certificate (self-signed)..."
openssl req -new -x509 -days $VALIDITY_DAYS -key "$CERTS_DIR/ca.key" -subj "/C=$COUNTRY/ST=$STATE/L=$CITY/O=$ORG/CN=BPA-CA" -out "$CERTS_DIR/ca.crt" 2>$null

# Step 3: Generate Server (Rust) private key
Write-Host "Generating server private key..."
openssl genrsa -out "$CERTS_DIR/server.key" 4096 2>$null

# Step 4: Generate Server CSR
Write-Host "Generating server CSR..."
openssl req -new -key "$CERTS_DIR/server.key" -subj "/C=$COUNTRY/ST=$STATE/L=$CITY/O=$ORG/CN=localhost" -addext "subjectAltName=DNS:localhost,DNS:core,IP:127.0.0.1" -out "$CERTS_DIR/server.csr" 2>$null

# Step 5: Sign Server CSR with CA
Write-Host "Signing server certificate with CA..."
$extFile = New-TemporaryFile
Set-Content -Path $extFile -Value "subjectAltName=DNS:localhost,DNS:core,IP:127.0.0.1"
openssl x509 -req -days $VALIDITY_DAYS -in "$CERTS_DIR/server.csr" -CA "$CERTS_DIR/ca.crt" -CAkey "$CERTS_DIR/ca.key" -CAcreateserial -out "$CERTS_DIR/server.crt" -extfile $extFile 2>$null
Remove-Item $extFile

# Step 6: Generate Client (Go) private key
Write-Host "Generating client private key..."
openssl genrsa -out "$CERTS_DIR/client.key" 4096 2>$null

# Step 7: Generate Client CSR
Write-Host "Generating client CSR..."
openssl req -new -key "$CERTS_DIR/client.key" -subj "/C=$COUNTRY/ST=$STATE/L=$CITY/O=$ORG/CN=bpa-client" -out "$CERTS_DIR/client.csr" 2>$null

# Step 8: Sign Client CSR with CA
Write-Host "Signing client certificate with CA..."
openssl x509 -req -days $VALIDITY_DAYS -in "$CERTS_DIR/client.csr" -CA "$CERTS_DIR/ca.crt" -CAkey "$CERTS_DIR/ca.key" -CAcreateserial -out "$CERTS_DIR/client.crt" 2>$null

# Step 9: Cleanup CSR files
Remove-Item "$CERTS_DIR\*.csr" -ErrorAction SilentlyContinue
Remove-Item "$CERTS_DIR\ca.srl" -ErrorAction SilentlyContinue

# Step 10: Verify certificates
Write-Host "Certificate generation complete!"
Write-Host ""
Write-Host "mTLS setup complete!"
