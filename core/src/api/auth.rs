use tonic::{Request, Response, Status};
use tracing::error;

use crate::bpa::auth_service_server::AuthService;
use crate::bpa::{
    LoginRequest, LoginResponse, RefreshRequest, RefreshResponse, RegisterStakeholderRequest,
    RegisterStakeholderResponse,
};
use crate::services::auth::AuthError;
use crate::BpaEngine;

#[tonic::async_trait]
impl AuthService for BpaEngine {
    async fn register_stakeholder(
        &self,
        request: Request<RegisterStakeholderRequest>,
    ) -> Result<Response<RegisterStakeholderResponse>, Status> {
        let req = request.into_inner();

        let encrypted_profile = self
            .encryption
            .encrypt(&req.profile_details)
            .map_err(|e| Status::internal(e.to_string()))?;

        match self
            .auth_service
            .register(
                req.email,
                req.password,
                req.role,
                encrypted_profile,
                req.aadhar_number,
                req.aadhar_document_base64,
            )
            .await
        {
            Ok(stakeholder_id) => Ok(Response::new(RegisterStakeholderResponse {
                stakeholder_id: stakeholder_id.to_string(),
                status: "SUCCESS".to_string(),
            })),
            Err(e) => {
                error!("Registration failed: {:?}", e);
                match e {
                    AuthError::UserExists => Err(Status::already_exists(e.to_string())),
                    _ => Err(Status::internal("Registration failed")),
                }
            }
        }
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();

        match self.auth_service.login(req.email, req.password).await {
            Ok((access_token, refresh_token, stakeholder_id, role)) => {
                Ok(Response::new(LoginResponse {
                    access_token,
                    refresh_token,
                    stakeholder_id: stakeholder_id.to_string(),
                    role,
                }))
            }
            Err(AuthError::InvalidCredentials) => {
                Err(Status::unauthenticated("Invalid credentials"))
            }
            Err(e) => {
                error!("Login failed: {:?}", e);
                Err(Status::internal("Internal server error"))
            }
        }
    }

    async fn refresh(
        &self,
        request: Request<RefreshRequest>,
    ) -> Result<Response<RefreshResponse>, Status> {
        let req = request.into_inner();

        match self.auth_service.refresh(req.refresh_token).await {
            Ok((access_token, refresh_token)) => Ok(Response::new(RefreshResponse {
                access_token,
                refresh_token,
            })),
            Err(AuthError::InvalidToken) => Err(Status::unauthenticated("Invalid or expired refresh token")),
            Err(e) => {
                error!("Refresh failed: {:?}", e);
                Err(Status::internal("Internal server error"))
            }
        }
    }
}
