use tonic::{Request, Response, Status};
use uuid::Uuid;
use std::str::FromStr;
use tracing::error;

use crate::BpaEngine;
use crate::bpa::bpa_service_server::BpaService;
use crate::bpa::{
    RegisterBatteryRequest as GrpcRegisterBatteryRequest, 
    RegisterBatteryResponse as GrpcRegisterBatteryResponse,
    GetBatteryRequest,
    GetBatteryResponse
};
use crate::services::registration::BatteryRegistrationRequest;

#[tonic::async_trait]
impl BpaService for BpaEngine {
    async fn register_battery(
        &self,
        request: Request<GrpcRegisterBatteryRequest>,
    ) -> Result<Response<GrpcRegisterBatteryResponse>, Status> {
        let req = request.into_inner();
        
        let manufacturer_id = Uuid::from_str(&req.manufacturer_id)
            .map_err(|_| Status::invalid_argument("Invalid manufacturer_id format"))?;
        
        let actor_id = Uuid::from_str(&req.actor_id)
            .map_err(|_| Status::invalid_argument("Invalid actor_id format"))?;

        let domain_req = BatteryRegistrationRequest {
            manufacturer_id,
            manufacturer_code: req.manufacturer_code,
            chemistry_type: req.chemistry_type,
            battery_category: req.battery_category,
            compliance_class: req.compliance_class,
            nominal_voltage: req.nominal_voltage,
            rated_capacity_kwh: req.rated_capacity_kwh,
            energy_density: req.energy_density,
            weight_kg: req.weight_kg,
            form_factor: req.form_factor,
            serial_number: req.serial_number,
            batch_number: req.batch_number,
            factory_code: req.factory_code,
            production_year: req.production_year as u16,
            sequence_number: req.sequence_number,
        };

        match self.registration.register_battery(domain_req, actor_id).await {
            Ok(res) => Ok(Response::new(GrpcRegisterBatteryResponse {
                bpan: res.bpan,
                static_hash: res.static_hash,
                registration_id: res.registration_id.to_string(),
                status: res.status,
            })),
            Err(e) => {
                error!("Failed to register battery: {:?}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }

    async fn get_battery(
        &self,
        request: Request<GetBatteryRequest>,
    ) -> Result<Response<GetBatteryResponse>, Status> {
        let req = request.into_inner();
        
        match self.registration.get_battery(&req.bpan).await {
            Ok(Some(battery)) => Ok(Response::new(GetBatteryResponse {
                bpan: req.bpan,
                chemistry_type: battery.chemistry_type,
                status: "ACTIVE".to_string(),
            })),
            Ok(None) => Err(Status::not_found("Battery not found")),
            Err(e) => {
                error!("Failed to get battery: {:?}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }
}
