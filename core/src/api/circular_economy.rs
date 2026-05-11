//! circular_economy.rs — gRPC service for reuse and recycling tracking

use tonic::{Request, Response, Status};
use std::sync::Arc;
use crate::BpaEngine;

pub mod ce_proto {
    tonic::include_proto!("bpa.circular_economy.v1");
}
pub use ce_proto::*;
pub use circular_economy_service_server::{CircularEconomyService, CircularEconomyServiceServer};

pub struct CircularEconomyServiceImpl {
    engine: Arc<BpaEngine>,
}

impl CircularEconomyServiceImpl {
    pub fn new(engine: Arc<BpaEngine>) -> Self {
        CircularEconomyServiceImpl { engine }
    }
}

#[tonic::async_trait]
impl CircularEconomyService for CircularEconomyServiceImpl {
    async fn certify_reuse(
        &self,
        request: Request<CertifyReuseRequest>,
    ) -> Result<Response<CertifyReuseResponse>, Status> {
        let req = request.into_inner();
        
        let cert_id = self.engine.reuse_service.certify_second_life(
            &req.bpan,
            req.soh_percent,
            &req.certified_by,
            &req.application,
            req.expected_years as u8,
        ).await.map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CertifyReuseResponse {
            certification_id: cert_id,
            certification_hash: "computed".to_string(), // In real impl, return actual hash
        }))
    }

    async fn record_recycling(
        &self,
        request: Request<RecordRecyclingRequest>,
    ) -> Result<Response<RecordRecyclingResponse>, Status> {
        let req = request.into_inner();
        let rr = req.recovery_rates.ok_or_else(|| Status::invalid_argument("missing recovery rates"))?;
        
        let rates = crate::services::recycling::RecoveryRates {
            lithium_percent: rr.lithium_percent,
            cobalt_percent: rr.cobalt_percent,
            nickel_percent: rr.nickel_percent,
            other_percent: rr.other_percent,
        };

        let cert_id = self.engine.recycling_service.record_recycling(
            &req.bpan,
            &req.recycled_by,
            &req.method,
            req.weight_kg,
            &req.standard,
            rates,
        ).await.map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(RecordRecyclingResponse {
            certification_id: cert_id,
            certification_hash: "computed".to_string(),
        }))
    }

    async fn get_metrics(
        &self,
        request: Request<GetMetricsRequest>,
    ) -> Result<Response<GetMetricsResponse>, Status> {
        let req = request.into_inner();
        
        // This is a bit simplified, usually you'd query both or combine
        let metrics = if !req.manufacturer_id.is_empty() {
            self.engine.recycling_repo.get_metrics_by_manufacturer(&req.manufacturer_id).await
        } else if !req.chemistry_type.is_empty() {
            self.engine.recycling_repo.get_metrics_by_chemistry(&req.chemistry_type).await
        } else {
            // Default to some global or error
            return Err(Status::invalid_argument("must provide manufacturer_id or chemistry_type"));
        }.map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(GetMetricsResponse {
            metrics: Some(CircularEconomyMetrics {
                battery_count: metrics.battery_count,
                avg_li_recovery: metrics.avg_li_recovery,
                avg_co_recovery: metrics.avg_co_recovery,
                avg_ni_recovery: metrics.avg_ni_recovery,
                total_weight_processed_kg: metrics.total_weight_processed_kg,
            }),
        }))
    }
}
