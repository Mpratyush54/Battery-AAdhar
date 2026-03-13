use std::fmt;

/// Unified error type for the Battery Pack Aadhaar core engine.
/// Every service returns this so the gRPC layer can map it to tonic::Status cleanly.
#[derive(Debug)]
pub enum BpaError {
    /// Database-level errors (sqlx)
    Database(sqlx::Error),
    /// Encryption/decryption failures
    Encryption(String),
    /// BPAN format or generation errors
    BpanFormat(String),
    /// Validation failures (business rules)
    Validation(String),
    /// Authorization / access control errors
    Unauthorized(String),
    /// Resource not found
    NotFound(String),
    /// Duplicate resource
    Conflict(String),
    /// Lifecycle state transition error
    InvalidStateTransition(String),
    /// Hash chain integrity violation
    IntegrityViolation(String),
    /// QR code generation/verification error
    QrError(String),
    /// Carbon footprint calculation error
    CarbonCalculation(String),
    /// Compliance check failure
    ComplianceFailure(String),
    /// Internal / unexpected error
    Internal(String),
}

impl fmt::Display for BpaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BpaError::Database(e) => write!(f, "Database error: {}", e),
            BpaError::Encryption(msg) => write!(f, "Encryption error: {}", msg),
            BpaError::BpanFormat(msg) => write!(f, "BPAN format error: {}", msg),
            BpaError::Validation(msg) => write!(f, "Validation error: {}", msg),
            BpaError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            BpaError::NotFound(msg) => write!(f, "Not found: {}", msg),
            BpaError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            BpaError::InvalidStateTransition(msg) => write!(f, "Invalid state transition: {}", msg),
            BpaError::IntegrityViolation(msg) => write!(f, "Integrity violation: {}", msg),
            BpaError::QrError(msg) => write!(f, "QR error: {}", msg),
            BpaError::CarbonCalculation(msg) => write!(f, "Carbon calculation error: {}", msg),
            BpaError::ComplianceFailure(msg) => write!(f, "Compliance failure: {}", msg),
            BpaError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for BpaError {}

impl From<sqlx::Error> for BpaError {
    fn from(err: sqlx::Error) -> Self {
        BpaError::Database(err)
    }
}

/// Convert BpaError into a tonic::Status for gRPC responses
impl From<BpaError> for tonic::Status {
    fn from(err: BpaError) -> Self {
        match &err {
            BpaError::Database(_) => tonic::Status::internal(err.to_string()),
            BpaError::Encryption(_) => tonic::Status::internal(err.to_string()),
            BpaError::BpanFormat(_) => tonic::Status::invalid_argument(err.to_string()),
            BpaError::Validation(_) => tonic::Status::invalid_argument(err.to_string()),
            BpaError::Unauthorized(_) => tonic::Status::permission_denied(err.to_string()),
            BpaError::NotFound(_) => tonic::Status::not_found(err.to_string()),
            BpaError::Conflict(_) => tonic::Status::already_exists(err.to_string()),
            BpaError::InvalidStateTransition(_) => tonic::Status::failed_precondition(err.to_string()),
            BpaError::IntegrityViolation(_) => tonic::Status::data_loss(err.to_string()),
            BpaError::QrError(_) => tonic::Status::internal(err.to_string()),
            BpaError::CarbonCalculation(_) => tonic::Status::internal(err.to_string()),
            BpaError::ComplianceFailure(_) => tonic::Status::failed_precondition(err.to_string()),
            BpaError::Internal(_) => tonic::Status::internal(err.to_string()),
        }
    }
}

pub type BpaResult<T> = Result<T, BpaError>;
