//! material_repo.rs — Persistence layer for BMCS (Battery Material Composition Sheet)
//!
//! Stores material composition rows (private fields as encrypted blob)
//! and creates hash-chain audit entries in `static_data_submission_log`.

use async_trait::async_trait;
use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::services::material::MaterialCompositionRow;

/// Error type for material repository operations.
#[derive(Debug)]
pub enum MaterialRepoError {
    NotFound(String),
    AlreadyExists(String),
    DatabaseError(String),
}

impl std::fmt::Display for MaterialRepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaterialRepoError::NotFound(m) => write!(f, "not found: {}", m),
            MaterialRepoError::AlreadyExists(m) => write!(f, "already exists: {}", m),
            MaterialRepoError::DatabaseError(m) => write!(f, "database error: {}", m),
        }
    }
}

impl std::error::Error for MaterialRepoError {}

/// Trait for material composition persistence.
#[async_trait]
pub trait MaterialRepository: Send + Sync {
    async fn insert(
        &self,
        row: &MaterialCompositionRow,
        submitter_id: Uuid,
        data_hash: &str,
    ) -> Result<String, MaterialRepoError>;

    async fn get_by_bpan(
        &self,
        bpan: &str,
    ) -> Result<Option<MaterialCompositionRow>, MaterialRepoError>;

    async fn update_field(
        &self,
        bpan: &str,
        field_name: &str,
        new_value: &str,
    ) -> Result<(), MaterialRepoError>;
}

/// Concrete Postgres-backed implementation.
pub struct MaterialRepositoryImpl {
    pool: PgPool,
}

impl MaterialRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Compute a hash-chain event hash: SHA-256(previous_event_hash || data_hash).
    fn chain_hash(previous: &str, data_hash: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(previous.as_bytes());
        hasher.update(data_hash.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[async_trait]
impl MaterialRepository for MaterialRepositoryImpl {
    /// Insert a new material composition row and create a submission log entry.
    #[instrument(name = "material_repo_insert", skip(self, row), fields(bpan = %row.bpan))]
    async fn insert(
        &self,
        row: &MaterialCompositionRow,
        submitter_id: Uuid,
        data_hash: &str,
    ) -> Result<String, MaterialRepoError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        // Insert into battery_material_composition
        sqlx::query(
            r#"
            INSERT INTO battery_material_composition
                (id, bpan, cathode_material, anode_material, electrolyte_type,
                 separator_material, lithium_content_g, cobalt_content_g,
                 nickel_content_g, recyclable_percentage, encrypted_details,
                 created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(id)
        .bind(&row.bpan)
        .bind(&row.cathode_material)
        .bind(&row.anode_material)
        .bind(&row.electrolyte_type)
        .bind(&row.separator_material)
        .bind(0.0_f64) // lithium_content_g placeholder (real value in encrypted_details)
        .bind(0.0_f64) // cobalt_content_g placeholder
        .bind(0.0_f64) // nickel_content_g placeholder
        .bind(row.recyclable_percentage)
        .bind(&row.encrypted_details)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                MaterialRepoError::AlreadyExists(format!(
                    "BMCS for BPAN {} already exists",
                    row.bpan
                ))
            } else {
                MaterialRepoError::DatabaseError(e.to_string())
            }
        })?;

        // Find previous event hash for this BPAN (hash-chain continuity)
        let previous_event_hash: String = sqlx::query_scalar(
            r#"
            SELECT event_hash FROM static_data_submission_log
            WHERE bpan = $1
            ORDER BY submitted_at DESC
            LIMIT 1
            "#,
        )
        .bind(&row.bpan)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| MaterialRepoError::DatabaseError(e.to_string()))?
        .unwrap_or_else(|| "0".repeat(64)); // genesis hash

        let event_hash = Self::chain_hash(&previous_event_hash, data_hash);

        // Insert submission log entry
        sqlx::query(
            r#"
            INSERT INTO static_data_submission_log
                (id, bpan, submitted_by, data_section, data_hash,
                 previous_event_hash, event_hash, submitted_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&row.bpan)
        .bind(submitter_id)
        .bind("BMCS")
        .bind(data_hash)
        .bind(&previous_event_hash)
        .bind(&event_hash)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| MaterialRepoError::DatabaseError(e.to_string()))?;

        info!(bpan = %row.bpan, event_hash = %event_hash, "BMCS inserted with audit trail");
        Ok(event_hash)
    }

    /// Retrieve material composition by BPAN.
    #[instrument(name = "material_repo_get", skip(self))]
    async fn get_by_bpan(
        &self,
        bpan: &str,
    ) -> Result<Option<MaterialCompositionRow>, MaterialRepoError> {
        let row = sqlx::query_as::<_, (String, String, String, String, String, f64, String)>(
            r#"
            SELECT bpan, cathode_material, anode_material, electrolyte_type,
                   separator_material, recyclable_percentage, encrypted_details
            FROM battery_material_composition
            WHERE bpan = $1
            LIMIT 1
            "#,
        )
        .bind(bpan)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| MaterialRepoError::DatabaseError(e.to_string()))?;

        Ok(row.map(
            |(bpan, cathode, anode, electrolyte, separator, recyclable, encrypted)| {
                MaterialCompositionRow {
                    bpan,
                    cathode_material: cathode,
                    anode_material: anode,
                    electrolyte_type: electrolyte,
                    separator_material: separator,
                    recyclable_percentage: recyclable,
                    encrypted_details: encrypted,
                }
            },
        ))
    }

    /// Update a single public field (for corrections).
    #[instrument(name = "material_repo_update", skip(self))]
    async fn update_field(
        &self,
        bpan: &str,
        field_name: &str,
        new_value: &str,
    ) -> Result<(), MaterialRepoError> {
        // Only allow updating known public columns to prevent SQL injection
        let column = match field_name {
            "cathode_material" => "cathode_material",
            "anode_material" => "anode_material",
            "electrolyte_type" => "electrolyte_type",
            "separator_material" => "separator_material",
            _ => {
                return Err(MaterialRepoError::DatabaseError(format!(
                    "field '{}' is not updatable via this method",
                    field_name
                )));
            }
        };

        let query = format!(
            "UPDATE battery_material_composition SET {} = $1 WHERE bpan = $2",
            column
        );

        let result = sqlx::query(&query)
            .bind(new_value)
            .bind(bpan)
            .execute(&self.pool)
            .await
            .map_err(|e| MaterialRepoError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(MaterialRepoError::NotFound(format!(
                "BMCS for BPAN {} not found",
                bpan
            )));
        }

        Ok(())
    }
}
