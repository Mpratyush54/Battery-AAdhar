use chrono::Utc;
use sqlx::{Pool, Postgres};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::errors::{BpaError, BpaResult};
use crate::services::hash_chain::HashChainService;

/// Manages end-of-life recycling workflows per BPA guidelines.
///
/// Recyclers are responsible for:
/// - Recording dismantling methods and material recovery outcomes
/// - Certifying the recycling process
/// - Generating EPR certificates for producers
#[derive(Clone)]
pub struct RecyclingService {
    pool: Pool<Postgres>,
}

impl RecyclingService {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// Record a recycling event for a battery.
    #[instrument(name = "record_recycling", skip(self))]
    pub async fn record_recycling(
        &self,
        bpan: &str,
        recycler_name: &str,
        recovered_material_percentage: f64,
        actor_id: Uuid,
    ) -> BpaResult<Uuid> {
        let now = Utc::now().naive_utc();
        let id = Uuid::new_v4();

        if recovered_material_percentage < 0.0 || recovered_material_percentage > 100.0 {
            return Err(BpaError::Validation(
                "Recovered material percentage must be between 0 and 100".into(),
            ));
        }

        let mut tx = self.pool.begin().await?;

        // Compute certificate hash
        let cert_data = format!("{}|{}|{:.2}|{}", bpan, recycler_name, recovered_material_percentage, now);
        let certificate_hash = HashChainService::compute_hash(&cert_data);

        // Insert recycling record
        sqlx::query("INSERT INTO recycling_records (id, bpan, recycler_name, recovered_material_percentage, certificate_hash, recycled_at) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(&id)
            .bind(bpan)
            .bind(recycler_name)
            .bind(recovered_material_percentage)
            .bind(&certificate_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        // Log the recycling certification
        let log_id = Uuid::new_v4();
        let ts_str = now.to_string();
        let event_hash = HashChainService::compute_entry_hash(
            &HashChainService::genesis_hash(),
            "RECORD_RECYCLING",
            bpan,
            &actor_id.to_string(),
            &ts_str,
        );

        sqlx::query("INSERT INTO recycling_certification_log (id, bpan, previous_event_hash, event_hash, recycler_hash, material_recovery_hash, certified_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&log_id)
            .bind(bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind(&HashChainService::compute_hash(recycler_name))
            .bind(&HashChainService::compute_hash(&format!("{:.2}", recovered_material_percentage)))
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        // Audit log
        let audit_id = Uuid::new_v4();
        sqlx::query("INSERT INTO audit_logs (id, actor_id, action, resource, previous_hash, entry_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&audit_id)
            .bind(&actor_id)
            .bind("RECORD_RECYCLING")
            .bind(bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        info!(
            "Recycling recorded for BPAN {}: {:.1}% material recovered by {}",
            bpan, recovered_material_percentage, recycler_name
        );
        Ok(id)
    }

    /// Get all recycling records for a battery.
    pub async fn get_recycling_records(&self, bpan: &str) -> BpaResult<Vec<RecyclingRecord>> {
        let rows: Vec<(Uuid, String, f64, String, chrono::NaiveDateTime)> = sqlx::query_as(
            "SELECT id, recycler_name, recovered_material_percentage, certificate_hash, recycled_at FROM recycling_records WHERE bpan = $1 ORDER BY recycled_at ASC"
        )
        .bind(bpan)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(id, recycler, pct, hash, at)| RecyclingRecord {
            id,
            bpan: bpan.to_string(),
            recycler_name: recycler,
            recovered_material_percentage: pct,
            certificate_hash: hash,
            recycled_at: at,
        }).collect())
    }

    /// Verify the certificate hash for a recycling record.
    pub async fn verify_certificate(&self, record_id: Uuid) -> BpaResult<bool> {
        let row: Option<(String, String, f64, String, chrono::NaiveDateTime)> = sqlx::query_as(
            "SELECT bpan, recycler_name, recovered_material_percentage, certificate_hash, recycled_at FROM recycling_records WHERE id = $1"
        )
        .bind(&record_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((bpan, recycler, pct, stored_hash, at)) => {
                let cert_data = format!("{}|{}|{:.2}|{}", bpan, recycler, pct, at);
                let computed_hash = HashChainService::compute_hash(&cert_data);
                Ok(computed_hash == stored_hash)
            }
            None => Err(BpaError::NotFound(format!(
                "No recycling record found with id {}",
                record_id
            ))),
        }
    }
}

/// A recycling record.
#[derive(Debug, Clone)]
pub struct RecyclingRecord {
    pub id: Uuid,
    pub bpan: String,
    pub recycler_name: String,
    pub recovered_material_percentage: f64,
    pub certificate_hash: String,
    pub recycled_at: chrono::NaiveDateTime,
}
