//! lifecycle.rs — gRPC service for ZK compliance verification
//!
//! Stub implementation. ZK proof generation wires in Day 12.

use tonic::{Request, Response, Status};

pub mod lifecycle_proto {
    tonic::include_proto!("bpa.lifecycle.v1");
}
pub use lifecycle_proto::*;
pub use lifecycle_service_server::{LifecycleService, LifecycleServiceServer};

pub struct LifecycleServiceImpl;

#[tonic::async_trait]
impl LifecycleService for LifecycleServiceImpl {
    async fn verify_operational(
        &self,
        _request: Request<VerifyOperationalRequest>,
    ) -> Result<Response<VerifyOperationalResponse>, Status> {
        Err(Status::unimplemented("VerifyOperational not yet implemented"))
    }

    async fn verify_recyclable(
        &self,
        _request: Request<VerifyRecyclableRequest>,
    ) -> Result<Response<VerifyRecyclableResponse>, Status> {
        Err(Status::unimplemented("VerifyRecyclable not yet implemented"))
    }

    async fn verify_signature(
        &self,
        _request: Request<VerifySignatureRequest>,
    ) -> Result<Response<VerifySignatureResponse>, Status> {
        Err(Status::unimplemented("VerifySignature not yet implemented"))
    }
}
