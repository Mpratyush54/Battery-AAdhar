use sqlx::{Pool, Postgres};
use uuid::Uuid;
use chrono::{Utc, Duration};
use jsonwebtoken::{encode, Header, Algorithm, EncodingKey};
use serde::{Deserialize, Serialize};
use bcrypt::{hash, verify, DEFAULT_COST};
use rand::{distributions::Alphanumeric, Rng};
use std::fmt;

#[derive(Debug)]
pub enum AuthError {
    DatabaseError(sqlx::Error),
    InvalidCredentials,
    InvalidToken,
    BcryptError(bcrypt::BcryptError),
    JwtError(jsonwebtoken::errors::Error),
    UserExists,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::DatabaseError(e) => write!(f, "Database error: {}", e),
            AuthError::InvalidCredentials => write!(f, "Invalid credentials"),
            AuthError::InvalidToken => write!(f, "Invalid token"),
            AuthError::BcryptError(e) => write!(f, "Bcrypt error: {}", e),
            AuthError::JwtError(e) => write!(f, "JWT error: {}", e),
            AuthError::UserExists => write!(f, "User already exists with this email"),
        }
    }
}

impl From<sqlx::Error> for AuthError {
    fn from(e: sqlx::Error) -> Self {
        AuthError::DatabaseError(e)
    }
}

impl From<bcrypt::BcryptError> for AuthError {
    fn from(e: bcrypt::BcryptError) -> Self {
        AuthError::BcryptError(e)
    }
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        AuthError::JwtError(e)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: usize,
}

#[derive(Clone)]
pub struct AuthService {
    db_pool: Pool<Postgres>,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(db_pool: Pool<Postgres>, jwt_secret: String) -> Self {
        Self { db_pool, jwt_secret }
    }

    pub async fn register(
        &self,
        email: String,
        password: String,
        role: String,
        encrypted_profile: String,
        aadhar_number: String,
        aadhar_document_base64: String,
    ) -> Result<Uuid, AuthError> {
        // Check if user exists
        let exists: (i64,) = sqlx::query_as("SELECT count(*) FROM stakeholder_credentials WHERE email = $1")
            .bind(&email)
            .fetch_one(&self.db_pool)
            .await?;

        if exists.0 > 0 {
            return Err(AuthError::UserExists);
        }

        let stakeholder_id = Uuid::new_v4();
        let password_hash = hash(password, DEFAULT_COST)?;

        let mut tx = self.db_pool.begin().await?;

        sqlx::query(
            "INSERT INTO stakeholders (id, role, encrypted_profile) VALUES ($1, $2, $3)"
        )
        .bind(stakeholder_id)
        .bind(&role)
        .bind(&encrypted_profile)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO stakeholder_credentials (stakeholder_id, email, password_hash) VALUES ($1, $2, $3)"
        )
        .bind(stakeholder_id)
        .bind(&email)
        .bind(&password_hash)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO stakeholder_kyc (stakeholder_id, aadhar_number, aadhar_document_base64) VALUES ($1, $2, $3)"
        )
        .bind(stakeholder_id)
        .bind(&aadhar_number)
        .bind(&aadhar_document_base64)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(stakeholder_id)
    }

    pub async fn login(&self, email: String, password: String) -> Result<(String, String, Uuid, String), AuthError> {
        let record: Option<(Uuid, String, String)> = sqlx::query_as(
            r#"
            SELECT c.stakeholder_id, c.password_hash, s.role 
            FROM stakeholder_credentials c
            JOIN stakeholders s ON s.id = c.stakeholder_id
            WHERE c.email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.db_pool)
        .await?;

        if let Some((stakeholder_id, password_hash, role)) = record {
            if verify(&password, &password_hash)? {
                let access_token = self.generate_jwt(stakeholder_id, &role)?;
                let refresh_token = self.generate_refresh_token(stakeholder_id).await?;
                return Ok((access_token, refresh_token, stakeholder_id, role));
            }
        }
        
        Err(AuthError::InvalidCredentials)
    }

    pub async fn refresh(&self, refresh_token: String) -> Result<(String, String), AuthError> {
        // Find the token
        let record: Option<(Uuid, Uuid, chrono::NaiveDateTime, String)> = sqlx::query_as(
            r#"
            SELECT r.id, r.stakeholder_id, r.expires_at, s.role 
            FROM refresh_tokens r
            JOIN stakeholders s ON s.id = r.stakeholder_id
            WHERE r.token = $1 AND r.revoked = FALSE
            "#,
        )
        .bind(refresh_token)
        .fetch_optional(&self.db_pool)
        .await?;

        if let Some((id, stakeholder_id, expires_at, role)) = record {
            if expires_at < Utc::now().naive_utc() {
                return Err(AuthError::InvalidToken);
            }

            // Revoke old token
            sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE id = $1")
                .bind(id)
                .execute(&self.db_pool)
                .await?;

            let access_token = self.generate_jwt(stakeholder_id, &role)?;
            let new_refresh_token = self.generate_refresh_token(stakeholder_id).await?;

            return Ok((access_token, new_refresh_token));
        }

        Err(AuthError::InvalidToken)
    }

    fn generate_jwt(&self, stakeholder_id: Uuid, role: &str) -> Result<String, AuthError> {
        let expiration = Utc::now()
            .checked_add_signed(Duration::minutes(15))
            .expect("valid timestamp")
            .timestamp() as usize;

        let claims = Claims {
            sub: stakeholder_id.to_string(),
            role: role.to_string(),
            exp: expiration,
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )?;

        Ok(token)
    }

    async fn generate_refresh_token(&self, stakeholder_id: Uuid) -> Result<String, AuthError> {
        let token: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect();

        let expires_at = Utc::now()
            .checked_add_signed(Duration::days(7))
            .expect("valid timestamp")
            .naive_utc();

        let id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO refresh_tokens (id, stakeholder_id, token, expires_at) VALUES ($1, $2, $3, $4)"
        )
        .bind(id)
        .bind(stakeholder_id)
        .bind(&token)
        .bind(expires_at)
        .execute(&self.db_pool)
        .await?;

        Ok(token)
    }
}
