//! battery.rs — gRPC service for battery registration, lookups, status updates
//!
//! Stub implementation. Business logic wires in Days 8–9.

use tonic::{Request, Response, Status};

pub mod battery_proto {
    tonic::include_proto!("bpa.battery.v1");
}
pub use battery_proto::*;
pub use battery_service_server::{BatteryService, BatteryServiceServer};

use crate::BpaEngine;
use std::sync::Arc;

pub struct BatteryServiceImpl {
    engine: Arc<BpaEngine>,
}

impl BatteryServiceImpl {
    pub fn new(engine: Arc<BpaEngine>) -> Self {
        BatteryServiceImpl { engine }
    }
}
#[tonic::async_trait]
impl BatteryService for BatteryServiceImpl {
    async fn register_battery(
        &self,
        _request: Request<RegisterBatteryRequest>,
    ) -> Result<Response<RegisterBatteryResponse>, Status> {
        Err(Status::unimplemented("RegisterBattery not yet implemented"))
    }

    async fn get_battery(
        &self,
        _request: Request<GetBatteryRequest>,
    ) -> Result<Response<GetBatteryResponse>, Status> {
        Err(Status::unimplemented("GetBattery not yet implemented"))
    }

    async fn update_battery_status(
        &self,
        _request: Request<UpdateBatteryStatusRequest>,
    ) -> Result<Response<UpdateBatteryStatusResponse>, Status> {
        Err(Status::unimplemented(
            "UpdateBatteryStatus not yet implemented",
        ))
    }
}
