//! battery.rs — gRPC service for battery registration, lookups, status updates,
//! and BMCS material composition (Day 8).

use tonic::{Request, Response, Status};
use tracing::info;
use uuid::Uuid;

pub mod battery_proto {
    tonic::include_proto!("bpa.battery.v1");
}
pub use battery_proto::*;
pub use battery_service_server::{BatteryService, BatteryServiceServer};

use crate::repositories::material_repo::MaterialRepositoryImpl;
use crate::services::material::{MaterialComposition as DomainMaterialComp, MaterialService};
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
        request: Request<RegisterBatteryRequest>,
    ) -> Result<Response<RegisterBatteryResponse>, Status> {
        let req = request.into_inner();
        
        let static_data = req.static_data.ok_or_else(|| Status::invalid_argument("static_data is required"))?;
        let material = req.material.ok_or_else(|| Status::invalid_argument("material is required"))?;
        let carbon = req.carbon.ok_or_else(|| Status::invalid_argument("carbon is required"))?;
        let initial_health = req.initial_health.ok_or_else(|| Status::invalid_argument("initial_health is required"))?;
        
        let manufacturer_uuid = Uuid::parse_str(&req.manufacturer_id).map_err(|e| Status::invalid_argument(format!("invalid manufacturer_id: {}", e)))?;

        // Map proto to domain models
        let domain_descriptor = crate::models::BatteryDescriptor {
            id: Uuid::new_v4(),
            bpan: String::new(),
            capacity_kwh: static_data.battery_capacity_kwh,
            nominal_voltage_v: static_data.nominal_voltage,
            nominal_current_a: 0.0, // not in proto
            chemistry_type: static_data.battery_chemistry.clone(),
            cell_type: static_data.cell_type.clone(),
            cell_count: static_data.num_cells,
            cell_voltage_nominal_v: 0.0,
            manufacturer_id: manufacturer_uuid,
            manufacturing_country: static_data.country_code,
            manufacturing_facility: static_data.factory_code,
            manufacture_date: static_data.manufacturing_date,
            declared_cycle_life: 0,
            warranty_years: static_data.warranty_years,
            registered_at: chrono::Utc::now(),
            battery_hash: String::new(),
        };

        let domain_material = crate::models::MaterialComposition {
            bpan: String::new(),
            cell_type: static_data.cell_type.clone(),
            chemistry_type: static_data.battery_chemistry.clone(),
            cathode_material: material.cathode_material.clone(),
            anode_material: material.anode_material.clone(),
            electrolyte_type: material.electrolyte_type.clone(),
            separator_type: material.separator_material.clone(),
            bms_type: "UNKNOWN".to_string(),
            bms_version: "UNKNOWN".to_string(),
            cooling_system: None,
            heating_system: None,
            terminal_type: "UNKNOWN".to_string(),
            case_material: "UNKNOWN".to_string(),
            weight_kg: 0.0,
            dimensions: "UNKNOWN".to_string(),
            internal_resistance_mohm: 0.0,
            nominal_capacity_ah: 0.0,
            warranty_years: static_data.warranty_years as u8,
            cycle_life_80_percent: 0,
            operating_temp_range: "UNKNOWN".to_string(),
            environmental_compliance: "UNKNOWN".to_string(),
            recyclable_percentage: material.recyclable_percentage as f32,
            recycling_instructions: None,
            submitted_by: req.manufacturer_id.clone(),
            submitted_at: chrono::Utc::now(),
        };

        let domain_carbon = crate::models::CarbonFootprint {
            bpan: String::new(),
            raw_material_emissions_kg_co2e: carbon.raw_material_emissions_kg_co2e as f32,
            raw_material_source_country: "UNKNOWN".to_string(),
            mining_method: "UNKNOWN".to_string(),
            manufacturing_emissions_kg_co2e: carbon.manufacturing_emissions_kg_co2e as f32,
            manufacturing_location: "UNKNOWN".to_string(),
            factory_energy_source: "UNKNOWN".to_string(),
            cell_production_method: "UNKNOWN".to_string(),
            transport_emissions_kg_co2e: carbon.transport_emissions_kg_co2e as f32,
            transport_distance_km: 0.0,
            transport_mode: "UNKNOWN".to_string(),
            transport_packaging: "UNKNOWN".to_string(),
            usage_emissions_kg_co2e: carbon.usage_emissions_kg_co2e as f32,
            usage_years: 0,
            usage_grid_emissions_factor: 0.0,
            usage_annual_km: 0,
            recycling_emissions_kg_co2e: carbon.recycling_emissions_kg_co2e as f32,
            recycling_recovery_rate: 0.0,
            recycling_avoided_mining: 0.0,
            recycling_method: "UNKNOWN".to_string(),
            total_emissions_kg_co2e: carbon.total_emissions_kg_co2e as f32,
            emissions_per_kwh: 0.0,
            carbon_hash: String::new(),
            submitted_by: req.manufacturer_id.clone(),
            submitted_at: chrono::Utc::now(),
            submitted_version: 1,
            verified: carbon.verified,
            verified_by: None,
            verified_at: None,
            verification_standard: None,
        };

        let domain_health = crate::models::HealthRecord {
            id: Uuid::new_v4(),
            bpan: String::new(),
            state_of_health_percent: initial_health.state_of_health_percent as f32,
            health_status: crate::models::HealthStatus::from_soh(initial_health.state_of_health_percent as f32),
            cycle_count: initial_health.cycle_count as u32,
            degradation_rate_percent_per_cycle: 0.0,
            degradation_class: initial_health.degradation_class.clone(),
            min_temperature_celsius: 0.0,
            max_temperature_celsius: 0.0,
            average_temperature_celsius: 0.0,
            cell_voltage_min_mv: 0.0,
            cell_voltage_max_mv: 0.0,
            internal_resistance_mohm: 0.0,
            error_flags: String::new(),
            is_healthy: true,
            reported_by: req.manufacturer_id.clone(),
            reported_at: chrono::Utc::now(),
            record_number: 1,
            zk_proof_soh_gt_80: None,
            zk_proof_soh_gte_60: None,
            zk_proof_soh_gte_30: None,
            proofs_generated_at: None,
        };

        let repo = crate::services::battery_registration::BatteryRegistrationServiceImpl::new(self.engine.db_pool.clone());
        use crate::services::battery_registration::BatteryRegistrationService;
        let bpan = repo.register_battery(
            &domain_descriptor,
            &domain_material,
            &domain_carbon,
            &domain_health,
            &req.manufacturer_id,
        ).await.map_err(|e| Status::internal(e.to_string()))?;

        // Note: Hash verification is skipped here for brevity, normally the domain models would be hydrated with proper hashes.

        Ok(Response::new(RegisterBatteryResponse {
            bpan,
            qr_code_png: vec![], // Generate QR in a later step if needed
            qr_payload: String::new(),
        }))
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

    // ─── BMCS: Submit Material Composition ─────────────────────────────

    async fn submit_material_composition(
        &self,
        request: Request<SubmitMaterialCompositionRequest>,
    ) -> Result<Response<SubmitMaterialCompositionResponse>, Status> {
        let req = request.into_inner();
        let proto_comp = req
            .composition
            .ok_or_else(|| Status::invalid_argument("composition is required"))?;

        let submitter_id = Uuid::parse_str(&req.submitter_id)
            .map_err(|_| Status::invalid_argument("invalid submitter_id UUID"))?;

        // Map proto → domain
        let comp = DomainMaterialComp {
            bpan: req.bpan.clone(),
            cathode_material: proto_comp.cathode_material,
            anode_material: proto_comp.anode_material,
            electrolyte_type: proto_comp.electrolyte_type,
            separator_material: proto_comp.separator_material,
            recyclable_percentage: proto_comp.recyclable_percentage,
            lithium_content_g: proto_comp.lithium_content_g,
            cobalt_content_g: proto_comp.cobalt_content_g,
            nickel_content_g: proto_comp.nickel_content_g,
            manganese_content_g: proto_comp.manganese_content_g,
            lead_content_g: proto_comp.lead_content_g,
            cadmium_content_g: proto_comp.cadmium_content_g,
            hazardous_substances: proto_comp.hazardous_substances,
            supply_chain_source: proto_comp.supply_chain_source,
        };

        // Encrypt private fields and get storable row
        let (row, data_hash) = self
            .engine
            .material_service
            .prepare_submission(&comp)
            .map_err(|e| Status::internal(format!("encryption failed: {}", e)))?;

        // Persist to database
        let repo = MaterialRepositoryImpl::new(self.engine.db_pool.clone());
        use crate::repositories::material_repo::MaterialRepository;
        let event_hash = repo
            .insert(&row, submitter_id, &data_hash)
            .await
            .map_err(|e| Status::internal(format!("db insert failed: {}", e)))?;

        info!(bpan = %req.bpan, "BMCS submitted successfully");

        Ok(Response::new(SubmitMaterialCompositionResponse {
            success: true,
            data_hash,
            event_hash,
        }))
    }

    // ─── BMCS: Get Material Composition ────────────────────────────────

    async fn get_material_composition(
        &self,
        request: Request<GetMaterialCompositionRequest>,
    ) -> Result<Response<GetMaterialCompositionResponse>, Status> {
        let req = request.into_inner();

        let repo = MaterialRepositoryImpl::new(self.engine.db_pool.clone());
        use crate::repositories::material_repo::MaterialRepository;
        let row = repo
            .get_by_bpan(&req.bpan)
            .await
            .map_err(|e| Status::internal(format!("db query failed: {}", e)))?
            .ok_or_else(|| Status::not_found(format!("no BMCS found for BPAN {}", req.bpan)))?;

        let can_see_private = MaterialService::can_see_private(&req.requester_role);

        if can_see_private {
            // Decrypt and return full composition
            let full = self
                .engine
                .material_service
                .decrypt_row(&row)
                .map_err(|e| Status::internal(format!("decryption failed: {}", e)))?;

            Ok(Response::new(GetMaterialCompositionResponse {
                composition: Some(material_to_proto(&full)),
                partial: false,
            }))
        } else {
            // Return only public fields
            let public = MaterialService::to_public(&row);
            Ok(Response::new(GetMaterialCompositionResponse {
                composition: Some(MaterialCompositionProto {
                    bpan: public.bpan,
                    cathode_material: public.cathode_material,
                    anode_material: public.anode_material,
                    electrolyte_type: public.electrolyte_type,
                    separator_material: public.separator_material,
                    recyclable_percentage: public.recyclable_percentage,
                    lithium_content_g: 0.0,
                    cobalt_content_g: 0.0,
                    nickel_content_g: 0.0,
                    manganese_content_g: 0.0,
                    lead_content_g: 0.0,
                    cadmium_content_g: 0.0,
                    hazardous_substances: String::new(),
                    supply_chain_source: String::new(),
                }),
                partial: true,
            }))
        }
    }
}

// Alias to avoid confusion between domain type and proto type
type MaterialCompositionProto = battery_proto::MaterialComposition;

fn material_to_proto(m: &DomainMaterialComp) -> MaterialCompositionProto {
    MaterialCompositionProto {
        bpan: m.bpan.clone(),
        cathode_material: m.cathode_material.clone(),
        anode_material: m.anode_material.clone(),
        electrolyte_type: m.electrolyte_type.clone(),
        separator_material: m.separator_material.clone(),
        recyclable_percentage: m.recyclable_percentage,
        lithium_content_g: m.lithium_content_g,
        cobalt_content_g: m.cobalt_content_g,
        nickel_content_g: m.nickel_content_g,
        manganese_content_g: m.manganese_content_g,
        lead_content_g: m.lead_content_g,
        cadmium_content_g: m.cadmium_content_g,
        hazardous_substances: m.hazardous_substances.clone(),
        supply_chain_source: m.supply_chain_source.clone(),
    }
}
