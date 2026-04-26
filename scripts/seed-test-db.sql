-- Schema setup (creates all tables from dbschma.txt)
-- For Day 5, we just create essential tables for testing

CREATE TABLE IF NOT EXISTS batteries (
    id UUID PRIMARY KEY,
    bpan VARCHAR(21) UNIQUE NOT NULL,
    manufacturer_id UUID,
    production_year INT,
    battery_category VARCHAR(50),
    compliance_class VARCHAR(50),
    static_hash VARCHAR(64),
    carbon_hash VARCHAR(64),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS battery_health (
    id UUID PRIMARY KEY,
    bpan VARCHAR(21) REFERENCES batteries(bpan),
    state_of_health FLOAT,
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS battery_ownership (
    id UUID PRIMARY KEY,
    bpan VARCHAR(21) REFERENCES batteries(bpan),
    owner_id VARCHAR(256),
    owner_type VARCHAR(50),
    start_time TIMESTAMP DEFAULT NOW(),
    end_time TIMESTAMP,
    transfer_reason VARCHAR(256)
);

CREATE TABLE IF NOT EXISTS audit_log (
    id UUID PRIMARY KEY,
    actor_id VARCHAR(256),
    action VARCHAR(256),
    resource VARCHAR(256),
    resource_id VARCHAR(256),
    details TEXT,
    entry_hash VARCHAR(64),
    entry_hash_prev VARCHAR(64),
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS compliance_violations (
    id UUID PRIMARY KEY,
    bpan VARCHAR(21) REFERENCES batteries(bpan),
    violation_type VARCHAR(256),
    severity VARCHAR(50),
    details TEXT,
    detected_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS battery_reuse (
    id UUID PRIMARY KEY,
    bpan VARCHAR(21) REFERENCES batteries(bpan),
    reuse_type VARCHAR(256),
    certifier_id VARCHAR(256),
    certified_at TIMESTAMP DEFAULT NOW(),
    expected_end_of_life TIMESTAMP
);

CREATE TABLE IF NOT EXISTS battery_recycling (
    id UUID PRIMARY KEY,
    bpan VARCHAR(21) REFERENCES batteries(bpan),
    recycler_id VARCHAR(256),
    recovered_percentage FLOAT,
    recovery_method VARCHAR(256),
    recycled_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS api_requests (
    id UUID PRIMARY KEY,
    actor_id VARCHAR(256),
    method VARCHAR(10),
    path VARCHAR(512),
    status_code INT,
    duration_ms INT,
    details JSONB,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Seed test data
INSERT INTO batteries (id, bpan, production_year, battery_category, compliance_class)
VALUES (
    'f47ac10b-58cc-4372-a567-0e02b2c3d479'::uuid,
    'MY008A6FKKKLC1DH80001',
    2025,
    'EV',
    'A'
) ON CONFLICT DO NOTHING;

INSERT INTO battery_health (id, bpan, state_of_health)
VALUES (
    'f47ac10b-58cc-4372-a567-0e02b2c3d480'::uuid,
    'MY008A6FKKKLC1DH80001',
    87.5
) ON CONFLICT DO NOTHING;
