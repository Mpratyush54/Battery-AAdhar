//! notification.rs — Encrypted notification service
//!
//! Handles notification creation, delivery tracking, and dead-letter queue.
//! All notification payloads are encrypted with the recipient's DEK.

use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::services::encryption::EncryptionService;

#[derive(Debug)]
pub enum NotificationError {
    NotFound(String),
    DatabaseError(String),
    EncryptionError(String),
}

impl std::fmt::Display for NotificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationError::NotFound(msg) => write!(f, "not found: {}", msg),
            NotificationError::DatabaseError(msg) => write!(f, "database error: {}", msg),
            NotificationError::EncryptionError(msg) => write!(f, "encryption error: {}", msg),
        }
    }
}

impl std::error::Error for NotificationError {}

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: Uuid,
    pub recipient_id: String,
    pub notification_type: String,
    pub encrypted_message: String,
    pub status: String,
    pub created_at: chrono::DateTime<Utc>,
    pub delivered_at: Option<chrono::DateTime<Utc>>,
    pub retry_count: i32,
}

#[async_trait]
pub trait NotificationService: Send + Sync {
    async fn create_notification(
        &self,
        recipient_id: &str,
        notification_type: &str,
        plaintext_message: &str,
    ) -> Result<String, NotificationError>;

    async fn mark_delivered(&self, notification_id: &str) -> Result<(), NotificationError>;

    async fn mark_read(&self, notification_id: &str) -> Result<(), NotificationError>;

    async fn get_pending_notifications(
        &self,
        recipient_id: &str,
        limit: i32,
    ) -> Result<Vec<Notification>, NotificationError>;

    async fn move_to_dlq(&self, notification_id: &str, error: &str) -> Result<(), NotificationError>;
}

pub struct NotificationServiceImpl {
    pool: PgPool,
    encryption: EncryptionService,
}

impl NotificationServiceImpl {
    pub fn new(pool: PgPool, encryption: EncryptionService) -> Self {
        NotificationServiceImpl { pool, encryption }
    }
}

#[async_trait]
impl NotificationService for NotificationServiceImpl {
    async fn create_notification(
        &self,
        recipient_id: &str,
        notification_type: &str,
        plaintext_message: &str,
    ) -> Result<String, NotificationError> {
        let encrypted = self
            .encryption
            .encrypt_bytes(plaintext_message.as_bytes())
            .map_err(|e| NotificationError::EncryptionError(e.to_string()))?;

        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO notifications 
            (id, recipient_id, notification_type, encrypted_message, status, created_at, retry_count)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(id)
        .bind(recipient_id)
        .bind(notification_type)
        .bind(encrypted)
        .bind("PENDING")
        .bind(now)
        .bind(0i32)
        .execute(&self.pool)
        .await
        .map_err(|e| NotificationError::DatabaseError(e.to_string()))?;

        tracing::info!(
            recipient = %recipient_id,
            type = %notification_type,
            id = %id,
            "notification created"
        );

        Ok(id.to_string())
    }

    async fn mark_delivered(&self, notification_id: &str) -> Result<(), NotificationError> {
        let id = Uuid::parse_str(notification_id)
            .map_err(|e| NotificationError::NotFound(format!("invalid UUID: {}", e)))?;

        sqlx::query(
            "UPDATE notifications SET status = 'DELIVERED', delivered_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| NotificationError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn mark_read(&self, notification_id: &str) -> Result<(), NotificationError> {
        let id = Uuid::parse_str(notification_id)
            .map_err(|e| NotificationError::NotFound(format!("invalid UUID: {}", e)))?;

        sqlx::query("UPDATE notifications SET status = 'READ' WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| NotificationError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn get_pending_notifications(
        &self,
        recipient_id: &str,
        limit: i32,
    ) -> Result<Vec<Notification>, NotificationError> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, String, String, chrono::DateTime<Utc>, Option<chrono::DateTime<Utc>>, i32)>(
            r#"
            SELECT id, recipient_id, notification_type, encrypted_message, status, created_at, delivered_at, retry_count
            FROM notifications
            WHERE recipient_id = $1 AND status = 'PENDING'
            ORDER BY created_at ASC
            LIMIT $2
            "#,
        )
        .bind(recipient_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| NotificationError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(
                |(id, recipient_id, notification_type, encrypted_message, status, created_at, delivered_at, retry_count)| {
                    Notification {
                        id,
                        recipient_id,
                        notification_type,
                        encrypted_message,
                        status,
                        created_at,
                        delivered_at,
                        retry_count,
                    }
                },
            )
            .collect())
    }

    async fn move_to_dlq(&self, notification_id: &str, error: &str) -> Result<(), NotificationError> {
        let id = Uuid::parse_str(notification_id)
            .map_err(|e| NotificationError::NotFound(format!("invalid UUID: {}", e)))?;

        let mut tx = self.pool.begin().await.map_err(|e| NotificationError::DatabaseError(e.to_string()))?;

        // Update notification status to FAILED
        sqlx::query("UPDATE notifications SET status = 'FAILED' WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| NotificationError::DatabaseError(e.to_string()))?;

        // Insert into dead_letter_queue
        sqlx::query(
            r#"
            INSERT INTO dead_letter_queue 
            (id, original_notification_id, error_message, created_at)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(error)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await
        .map_err(|e| NotificationError::DatabaseError(e.to_string()))?;

        tx.commit().await.map_err(|e| NotificationError::DatabaseError(e.to_string()))?;

        tracing::warn!(id = %notification_id, error = %error, "notification moved to DLQ");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Integration tests require real Postgres
}
