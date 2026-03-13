use chrono::Utc;
use sqlx::{Pool, Postgres};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::errors::BpaResult;
use crate::services::hash_chain::HashChainService;

/// Manages battery reuse/repurpose workflows per BPA guidelines.
///
/// When a battery's SoH drops below 80% but remains above 60%,
/// it becomes a reuse candidate. This service handles:
/// - Certification of reuse applications
/// - Issuing new BPANs for repurposed batteries (if materially altered)
/// - Recording second-life application details
#[derive(Clone)]
pub struct ReuseService {
    pool: Pool<Postgres>,
}

impl ReuseService {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// Record a reuse certification for a battery.
    #[instrument(name = "certify_reuse", skip(self))]
    pub async fn certify_reuse(
        &self,
        bpan: &str,
        reuse_application: &str,
        certified_by: &str,
        actor_id: Uuid,
    ) -> BpaResult<Uuid> {
        let now = Utc::now().naive_utc();
        let id = Uuid::new_v4();

        let mut tx = self.pool.begin().await?;

        // Insert reuse history record
        sqlx::query("INSERT INTO reuse_history (id, bpan, reuse_application, certified_by, certified_at) VALUES ($1, $2, $3, $4, $5)")
            .bind(&id)
            .bind(bpan)
            .bind(reuse_application)
            .bind(certified_by)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        // Log the reuse certification
        let log_id = Uuid::new_v4();
        let ts_str = now.to_string();
        let event_hash = HashChainService::compute_entry_hash(
            &HashChainService::genesis_hash(),
            "CERTIFY_REUSE",
            bpan,
            &actor_id.to_string(),
            &ts_str,
        );

        sqlx::query("INSERT INTO reuse_certification_log (id, bpan, previous_event_hash, event_hash, application_type, certifier_hash, certified_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&log_id)
            .bind(bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind(reuse_application)
            .bind(&HashChainService::compute_hash(certified_by))
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        // Audit log
        let audit_id = Uuid::new_v4();
        sqlx::query("INSERT INTO audit_logs (id, actor_id, action, resource, previous_hash, entry_hash, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&audit_id)
            .bind(&actor_id)
            .bind("CERTIFY_REUSE")
            .bind(bpan)
            .bind(&HashChainService::genesis_hash())
            .bind(&event_hash)
            .bind(&now)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        info!("Reuse certified for BPAN {}: {} by {}", bpan, reuse_application, certified_by);
        Ok(id)
    }

    /// Get the reuse history for a battery.
    pub async fn get_reuse_history(&self, bpan: &str) -> BpaResult<Vec<ReuseRecord>> {
        let rows: Vec<(Uuid, String, String, chrono::NaiveDateTime)> = sqlx::query_as(
            "SELECT id, reuse_application, certified_by, certified_at FROM reuse_history WHERE bpan = $1 ORDER BY certified_at ASC"
        )
        .bind(bpan)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(id, app, cert, at)| ReuseRecord {
            id,
            bpan: bpan.to_string(),
            reuse_application: app,
            certified_by: cert,
            certified_at: at,
        }).collect())
    }
}

/// A reuse record.
#[derive(Debug, Clone)]
pub struct ReuseRecord {
    pub id: Uuid,
    pub bpan: String,
    pub reuse_application: String,
    pub certified_by: String,
    pub certified_at: chrono::NaiveDateTime,
}
