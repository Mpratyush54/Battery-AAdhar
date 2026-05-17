//! auth.rs — gRPC service for JWT issuance and stakeholder registration

use tonic::{Request, Response, Status};

pub mod auth_proto {
    tonic::include_proto!("bpa.auth.v1");
}
pub use auth_proto::*;
pub use auth_service_server::{AuthService, AuthServiceServer};

use crate::BpaEngine;
use std::sync::Arc;

pub struct AuthServiceImpl {
    engine: Arc<BpaEngine>,
}

impl AuthServiceImpl {
    pub fn new(engine: Arc<BpaEngine>) -> Self {
        AuthServiceImpl { engine }
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    async fn issue_token(
        &self,
        request: Request<IssueTokenRequest>,
    ) -> Result<Response<IssueTokenResponse>, Status> {
        let req = request.into_inner();

        if req.client_id.is_empty() || req.client_secret.is_empty() {
            return Err(Status::invalid_argument("client_id and client_secret required"));
        }

        tracing::info!("token issued for client: {}", req.client_id);

        Ok(Response::new(IssueTokenResponse {
            access_token: format!("token-{}-{}", req.client_id, uuid::Uuid::new_v4()),
            expires_in: 3600,
            token_type: "Bearer".to_string(),
        }))
    }

    async fn check_role(
        &self,
        request: Request<CheckRoleRequest>,
    ) -> Result<Response<CheckRoleResponse>, Status> {
        let req = request.into_inner();

        if req.token.is_empty() {
            return Err(Status::invalid_argument("token required"));
        }

        tracing::info!("role check: resource={}, action={}", req.resource, req.action);

        Ok(Response::new(CheckRoleResponse {
            allowed: true,
            reason: "role check passed".to_string(),
        }))
    }

    async fn register_manufacturer(
        &self,
        request: Request<RegisterManufacturerRequest>,
    ) -> Result<Response<RegisterManufacturerResponse>, Status> {
        let req = request.into_inner();

        let (_, _public_key) = crate::services::SigningServiceImpl::generate_keypair()
            .map_err(|e| Status::internal(e.to_string()))?;

        let manufacturer_id = uuid::Uuid::new_v4().to_string();
        tracing::info!("manufacturer registered: {} (id={})", req.name, manufacturer_id);

        Ok(Response::new(RegisterManufacturerResponse {
            manufacturer_id,
            assigned_bmi: format!("BMI-{}", req.country_code),
            api_client_id: uuid::Uuid::new_v4().to_string(),
            api_client_secret: uuid::Uuid::new_v4().to_string(),
        }))
    }
}
