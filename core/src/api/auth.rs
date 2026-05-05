//! auth.rs — gRPC service for JWT issuance and stakeholder registration
//!
//! Stub implementation. Real auth wires in Day 15 (RS256 validation).

use tonic::{Request, Response, Status};

pub mod auth_proto {
    tonic::include_proto!("bpa.auth.v1");
}
pub use auth_proto::*;
pub use auth_service_server::{AuthService, AuthServiceServer};

use crate::BpaEngine;
use std::sync::Arc;

pub struct AuthServiceImpl {
    _engine: Arc<BpaEngine>,
}

impl AuthServiceImpl {
    pub fn new(engine: Arc<BpaEngine>) -> Self {
        AuthServiceImpl { _engine: engine }
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    async fn issue_token(
        &self,
        _request: Request<IssueTokenRequest>,
    ) -> Result<Response<IssueTokenResponse>, Status> {
        // TODO Day 15: JWT token issuance
        Err(Status::unimplemented("JWT issuance on Day 15"))
    }

    async fn check_role(
        &self,
        _request: Request<CheckRoleRequest>,
    ) -> Result<Response<CheckRoleResponse>, Status> {
        // TODO Day 15: Role verification
        Err(Status::unimplemented("Role checking on Day 15"))
    }

    async fn register_manufacturer(
        &self,
        request: Request<RegisterManufacturerRequest>,
    ) -> Result<Response<RegisterManufacturerResponse>, Status> {
        let req = request.into_inner();

        // Generate keypair for manufacturer
        let (_, _public_key) = crate::services::SigningServiceImpl::generate_keypair()
            .map_err(|e| Status::internal(e.to_string()))?;

        // TODO Day 7: Store in DB
        tracing::info!("manufacturer registered: {}", req.name);

        Ok(Response::new(RegisterManufacturerResponse {
            manufacturer_id: uuid::Uuid::new_v4().to_string(),
            assigned_bmi: format!("BMI-{}", req.country_code),
            api_client_id: uuid::Uuid::new_v4().to_string(),
            api_client_secret: uuid::Uuid::new_v4().to_string(),
        }))
    }
}
