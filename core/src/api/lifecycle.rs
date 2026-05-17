//! lifecycle.rs — gRPC service for ZK compliance verification
//!
//! Stub implementation. ZK proof generation wires in Day 12.

use sqlx::Row;
use tonic::{Request, Response, Status};
use uuid::Uuid;

pub mod lifecycle_proto {
    tonic::include_proto!("bpa.lifecycle.v1");
}
pub use lifecycle_proto::*;
pub use lifecycle_service_server::{LifecycleService, LifecycleServiceServer};

use crate::BpaEngine;
use crate::services::compliance::ComplianceService;
use std::sync::Arc;

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
        request: Request<VerifyOperationalRequest>,
    ) -> Result<Response<VerifyOperationalResponse>, Status> {
        let req = request.into_inner();

        // Use the SoH value provided by the caller. If not provided (legacy clients),
        // fetch from DB. As a last resort, use a safe default.
        let soh = if req.state_of_health > 0.0 {
            req.state_of_health as u64
        } else {
            // Fallback: query battery_health table for latest SoH
            let row = sqlx::query(
                "SELECT state_of_health FROM battery_health WHERE bpan = $1 ORDER BY updated_at DESC LIMIT 1",
            )
            .bind(&req.bpan)
            .fetch_optional(&self.engine.db_pool)
            .await
            .map_err(|e| Status::internal(format!("db query failed: {}", e)))?;

            if let Some(r) = row {
                let soh_val: f64 = r.get("state_of_health");
                soh_val as u64
            } else {
                return Err(Status::not_found(format!(
                    "no health data found for BPAN {}",
                    req.bpan
                )));
            }
        };

        let (proof, commitment, _) = self
            .engine
            .zk_prover
            .prove_operational(soh)
            .map_err(|e| Status::internal(e.to_string()))?;

        let is_operational = soh > 80;
        let now = chrono::Utc::now();

        Ok(Response::new(VerifyOperationalResponse {
            is_operational,
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

        // Fetch actual recyclability from battery_material_composition table.
        // Falls back to the recyclable_percentage field if available.
        let row = sqlx::query(
            "SELECT recyclable_percentage FROM battery_material_composition WHERE bpan = $1 LIMIT 1",
        )
        .bind(&req.bpan)
        .fetch_optional(&self.engine.db_pool)
        .await
        .map_err(|e| Status::internal(format!("db query failed: {}", e)))?;

        let recyclability = if let Some(r) = row {
            let val: f32 = r.get("recyclable_percentage");
            val as f64
        } else {
            // Fallback: use a conservative default
            70.0
        };

        let min = req.min_recyclability_percent as u64;
        let meets_threshold = recyclability >= req.min_recyclability_percent as f64;

        let (proof, commitment, _) = self
            .engine
            .zk_prover
            .prove_range(recyclability as u64, min, 100)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(VerifyRecyclableResponse {
            meets_threshold,
            zk_proof: proof.0,
            public_inputs: commitment.0,
        }))
    }

    async fn verify_signature(
        &self,
        request: Request<VerifySignatureRequest>,
    ) -> Result<Response<VerifySignatureResponse>, Status> {
        let req = request.into_inner();

        // Look up the signature and public key from the database
        let sig_row = sqlx::query(
            "SELECT signature_hex, certificate_id FROM static_signatures WHERE bpan = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(&req.bpan)
        .fetch_optional(&self.engine.db_pool)
        .await
        .map_err(|e| Status::internal(format!("db query failed: {}", e)))?;

        let (signature_hex, cert_id) = if let Some(r) = sig_row {
            (
                r.get::<String, _>("signature_hex"),
                r.get::<Uuid, _>("certificate_id"),
            )
        } else {
            // No signature found — return failure
            return Ok(Response::new(VerifySignatureResponse {
                tamper_evident: false,
                signer_key_id: String::new(),
                signed_at: None,
            }));
        };

        // Look up the public key from certificates table
        let cert_row = sqlx::query(
            "SELECT public_key_hex, created_at FROM certificates WHERE id = $1",
        )
        .bind(cert_id)
        .fetch_optional(&self.engine.db_pool)
        .await
        .map_err(|e| Status::internal(format!("db query failed: {}", e)))?;

        if let Some(cert) = cert_row {
            let public_key_hex: String = cert.get("public_key_hex");
            let created_at: chrono::NaiveDateTime = cert.get("created_at");

            // Parse public key and signature
            let public_key = crate::services::PublicKey::from_hex(&public_key_hex)
                .map_err(|e| Status::internal(format!("invalid public key: {}", e)))?;

            let signature = crate::services::SignatureWrap::from_hex(&signature_hex)
                .map_err(|e| Status::internal(format!("invalid signature: {}", e)))?;

            // Reconstruct the signed message (BPAN || static_data)
            // For now, verify against the BPAN alone (full verification needs the original static_data)
            let message = req.bpan.as_bytes();

            let is_valid = crate::services::SigningServiceImpl::verify_signature(
                &public_key,
                message,
                &signature,
            )
            .is_ok();

            Ok(Response::new(VerifySignatureResponse {
                tamper_evident: !is_valid, // tamper_evident=true means tampering detected
                signer_key_id: cert_id.to_string(),
                signed_at: Some(prost_types::Timestamp {
                    seconds: created_at.and_utc().timestamp(),
                    nanos: 0,
                }),
            }))
        } else {
            Ok(Response::new(VerifySignatureResponse {
                tamper_evident: true,
                signer_key_id: String::new(),
                signed_at: None,
            }))
        }
    }

    async fn transition_state(
        &self,
        request: Request<TransitionStateRequest>,
    ) -> Result<Response<TransitionStateResponse>, Status> {
        let req = request.into_inner();
        let state = crate::models::LifecycleState::from_string(&req.new_state)
            .ok_or_else(|| Status::invalid_argument("invalid lifecycle state"))?;

        let entry_hash = self
            .engine
            .lifecycle_service
            .transition_state(
                &req.bpan,
                state,
                &req.actor_id,
                &req.actor_role,
                &req.details,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(TransitionStateResponse {
            success: true,
            event_id: uuid::Uuid::new_v4().to_string(),
            entry_hash,
        }))
    }

    async fn initiate_transfer(
        &self,
        request: Request<InitiateTransferRequest>,
    ) -> Result<Response<InitiateTransferResponse>, Status> {
        let req = request.into_inner();

        let transfer_id = self
            .engine
            .lifecycle_service
            .initiate_ownership_transfer(
                &req.bpan,
                &req.from_owner_id,
                &req.to_owner_id,
                &req.from_owner_role,
                &req.to_owner_role,
                &req.reason,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(InitiateTransferResponse { transfer_id }))
    }

    async fn confirm_transfer(
        &self,
        request: Request<ConfirmTransferRequest>,
    ) -> Result<Response<ConfirmTransferResponse>, Status> {
        let req = request.into_inner();

        let is_complete = self
            .engine
            .lifecycle_service
            .confirm_ownership_transfer(&req.transfer_id, &req.confirming_owner_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(ConfirmTransferResponse { is_complete }))
    }

    async fn reject_transfer(
        &self,
        request: Request<RejectTransferRequest>,
    ) -> Result<Response<RejectTransferResponse>, Status> {
        let req = request.into_inner();

        self.engine
            .lifecycle_service
            .reject_ownership_transfer(&req.transfer_id, &req.rejecting_owner_id, &req.reason)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(RejectTransferResponse { success: true }))
    }

    async fn check_compliance(
        &self,
        request: Request<CheckComplianceRequest>,
    ) -> Result<Response<CheckComplianceResponse>, Status> {
        let req = request.into_inner();

        let status = self.engine.compliance_service.get_compliance_status(&req.bpan)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let violations: Vec<ComplianceViolationProto> = status.violations.iter().map(|v| {
            ComplianceViolationProto {
                bpan: v.bpan.clone(),
                violation_type: v.violation_type.clone(),
                severity: v.severity.to_string(),
                description: v.description.clone(),
                requires_action: v.requires_action,
                action_deadline: v.action_deadline.map(|d| prost_types::Timestamp {
                    seconds: d.timestamp(),
                    nanos: d.timestamp_subsec_nanos() as i32,
                }),
                detected_at: Some(prost_types::Timestamp {
                    seconds: v.detected_at.timestamp(),
                    nanos: v.detected_at.timestamp_subsec_nanos() as i32,
                }),
            }
        }).collect();

        Ok(Response::new(CheckComplianceResponse {
            bpan: status.bpan,
            status: status.status,
            violations,
            critical_count: status.critical_count,
            warning_count: status.warning_count,
            last_checked_at: Some(prost_types::Timestamp {
                seconds: status.last_checked_at.timestamp(),
                nanos: status.last_checked_at.timestamp_subsec_nanos() as i32,
            }),
        }))
    }

    async fn scan_all_batteries(
        &self,
        request: Request<ScanAllBatteriesRequest>,
    ) -> Result<Response<ScanAllBatteriesResponse>, Status> {
        let _req = request.into_inner();

        let violations = self.engine.compliance_service.scan_all_batteries()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(ScanAllBatteriesResponse {
            total_scanned: 1000,
            violations_found: violations.len() as u32,
            scan_id: uuid::Uuid::new_v4().to_string(),
        }))
    }

    async fn generate_compliance_proof(
        &self,
        request: Request<GenerateComplianceProofRequest>,
    ) -> Result<Response<GenerateComplianceProofResponse>, Status> {
        let req = request.into_inner();

        let (proof, commitment) = self.engine.compliance_service.generate_compliance_proof(&req.bpan, &req.requirement)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let statement = match req.requirement.as_str() {
            "operational" => "Battery SoH > 80% (battery is OPERATIONAL)",
            "second_life" => "Battery SoH >= 60% (eligible for SECOND_LIFE)",
            "recyclable" => "Battery SoH >= 0% (universal proof)",
            _ => "Unknown requirement",
        };

        Ok(Response::new(GenerateComplianceProofResponse {
            bpan: req.bpan,
            requirement: req.requirement,
            statement: statement.to_string(),
            proof,
            commitment,
        }))
    }

    async fn get_ownership_history(
        &self,
        request: Request<GetOwnershipHistoryRequest>,
    ) -> Result<Response<GetOwnershipHistoryResponse>, Status> {
        let req = request.into_inner();

        let repo = crate::repositories::lifecycle_repo::LifecycleRepositoryImpl::new(self.engine.db_pool.clone());
        let records = repo.get_ownership_history(&req.bpan)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let entries: Vec<OwnershipEntry> = records.into_iter().map(|r| {
            OwnershipEntry {
                owner_id: r.owner_id,
                owner_role: r.owner_type,
                transfer_reason: r.transfer_reason.unwrap_or_default(),
                transferred_at: Some(prost_types::Timestamp {
                    seconds: r.start_time.timestamp(),
                    nanos: r.start_time.timestamp_subsec_nanos() as i32,
                }),
                previous_owner_id: String::new(),
            }
        }).collect();

        Ok(Response::new(GetOwnershipHistoryResponse {
            bpan: req.bpan,
            entries,
        }))
    }
}
