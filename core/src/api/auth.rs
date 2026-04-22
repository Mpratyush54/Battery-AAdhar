//! auth.rs — gRPC service for JWT issuance and stakeholder registration
//!
//! Stub implementation. Real auth wires in Day 15 (RS256 validation).

use tonic::{Request, Response, Status};

pub mod auth_proto {
    tonic::include_proto!("bpa.auth.v1");
}
pub use auth_proto::*;
pub use auth_service_server::{AuthService, AuthServiceServer};

pub struct AuthServiceImpl;

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    async fn issue_token(
        &self,
        _request: Request<IssueTokenRequest>,
    ) -> Result<Response<IssueTokenResponse>, Status> {
        Err(Status::unimplemented("IssueToken not yet implemented"))
    }

    async fn check_role(
        &self,
        _request: Request<CheckRoleRequest>,
    ) -> Result<Response<CheckRoleResponse>, Status> {
        Err(Status::unimplemented("CheckRole not yet implemented"))
    }

    async fn register_manufacturer(
        &self,
        _request: Request<RegisterManufacturerRequest>,
    ) -> Result<Response<RegisterManufacturerResponse>, Status> {
        Err(Status::unimplemented("RegisterManufacturer not yet implemented"))
    }
}
