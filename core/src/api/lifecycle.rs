//! lifecycle.rs — gRPC service for ZK compliance verification
//!
//! Stub implementation. ZK proof generation wires in Day 12.

use tonic::{Request, Response, Status};

pub mod lifecycle_proto {
    tonic::include_proto!("bpa.lifecycle.v1");
}
pub use lifecycle_proto::*;
pub use lifecycle_service_server::{LifecycleService, LifecycleServiceServer};

use std::sync::Arc;
use crate::BpaEngine;

pub struct LifecycleServiceImpl {
    engine: Arc<BpaEngine>,
}

impl LifecycleServiceImpl {
    pub fn new(engine: Arc<BpaEngine>) -> Self {
        LifecycleServiceImpl { engine }
    }
}

#[tonic::async_trait]
impl LifecycleService for LifecycleServiceImpl {
    async fn verify_operational(
        &self,
        _request: Request<VerifyOperationalRequest>,
    ) -> Result<Response<VerifyOperationalResponse>, Status> {
        // TODO Day 7: Retrieve actual SoH from battery_health table
        // For now, generate proof for a test value
        let test_soh = 87u64;

        let (proof, commitment, _) = self.engine.zk_prover
            .prove_operational(test_soh)
            .map_err(|e| Status::internal(e.to_string()))?;

        let now = chrono::Utc::now();

        Ok(Response::new(VerifyOperationalResponse {
            is_operational: true,
            zk_proof: proof.0,
            public_inputs: commitment.0,
            proof_issued_at: Some(prost_types::Timestamp {
                seconds: now.timestamp(),
                nanos: now.timestamp_subsec_nanos() as i32,
            }),
            proof_valid_until_unix: (now + chrono::Duration::days(30)).timestamp(),
        }))
    }

    async fn verify_recyclable(
        &self,
        request: Request<VerifyRecyclableRequest>,
    ) -> Result<Response<VerifyRecyclableResponse>, Status> {
        let req = request.into_inner();

        // TODO Day 7: Retrieve actual recyclability % from DB
        let test_recyclability = 75.0;

        let min = req.min_recyclability_percent as u64;
        let (proof, commitment, _) = self.engine.zk_prover
            .prove_range(test_recyclability as u64, min, 100)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(VerifyRecyclableResponse {
            meets_threshold: true,
            zk_proof: proof.0,
            public_inputs: commitment.0,
        }))
    }

    async fn verify_signature(
        &self,
        request: Request<VerifySignatureRequest>,
    ) -> Result<Response<VerifySignatureResponse>, Status> {
        let req = request.into_inner();

        // TODO Day 7: Retrieve signature and manufacturer public key from DB
        tracing::info!("signature verification requested for {}", req.bpan);

        Ok(Response::new(VerifySignatureResponse {
            tamper_evident: true,
            signer_key_id: "mfr-key-1".to_string(),
            signed_at: Some(prost_types::Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: 0,
            }),
        }))
    }
}
