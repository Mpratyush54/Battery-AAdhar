//! battery.rs — gRPC service for battery registration, lookups, status updates,
//! and BMCS material composition (Day 8).
//!
//! Registration uses the spec-compliant `RegistrationService` which generates
//! proper 21-char BPANs via `BpanGenerator`, encrypts identifiers, and writes
//! 6 tables in a single transaction with audit logging.

use sqlx::Row;
use std::sync::Arc;
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
use crate::services::registration::{BatteryRegistrationRequest, RegistrationService};
use crate::BpaEngine;

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

        let static_data = req
            .static_data
            .ok_or_else(|| Status::invalid_argument("static_data is required"))?;

        let manufacturer_uuid = Uuid::parse_str(&req.manufacturer_id)
            .map_err(|e| Status::invalid_argument(format!("invalid manufacturer_id: {}", e)))?;

        // Parse production year from manufacturing_date (YYYYMMDD format)
        let production_year = if static_data.manufacturing_date.len() >= 4 {
            static_data.manufacturing_date[..4]
                .parse::<u16>()
                .map_err(|_| Status::invalid_argument("invalid manufacturing_date"))?
        } else {
            2025
        };

        // Derive serial_number from manufacturing_date + factory_code + sequential_number
        // This ensures uniqueness while using available proto fields
        let serial_number = format!(
            "{}{}{}",
            static_data.manufacturing_date, static_data.factory_code, static_data.sequential_number
        );

        // Use sequential_number as the 2-char sequence for BPAN generation
        let sequence_number = if static_data.sequential_number.len() >= 2 {
            static_data.sequential_number[..2].to_string()
        } else {
            "01".to_string()
        };

        // Derive battery_category from chemistry and capacity
        let battery_category = derive_battery_category(
            &static_data.battery_chemistry,
            static_data.battery_capacity_kwh,
        );

        // Build the registration request for the spec-compliant service
        let reg_request = BatteryRegistrationRequest {
            manufacturer_id: manufacturer_uuid,
            manufacturer_code: static_data.manufacturer_code.clone(),
            chemistry_type: static_data.battery_chemistry.clone(),
            battery_category,
            compliance_class: "AIS-156".to_string(), // Default compliance class
            nominal_voltage: static_data.nominal_voltage as f64,
            rated_capacity_kwh: static_data.battery_capacity_kwh as f64,
            energy_density: 0.0, // Not provided in proto — can be computed later
            weight_kg: static_data.weight_kg as f64,
            form_factor: static_data.cell_type.clone(),
            serial_number,
            batch_number: static_data.factory_code.clone(),
            factory_code: static_data.factory_code.clone(),
            production_year,
            sequence_number,
        };

        // Call the spec-compliant registration service
        let reg_service =
            RegistrationService::new(self.engine.db_pool.clone(), self.engine.encryption.clone());

        let reg_response = reg_service
            .register_battery(reg_request, manufacturer_uuid)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "battery registration failed");
                Status::internal(format!("registration failed: {}", e))
            })?;

        // Also store material composition if provided
        if req.material.is_some() {
            let mat = req.material.unwrap();
            let domain_material = DomainMaterialComp {
                bpan: reg_response.bpan.clone(),
                cathode_material: mat.cathode_material,
                anode_material: mat.anode_material,
                electrolyte_type: mat.electrolyte_type,
                separator_material: mat.separator_material,
                recyclable_percentage: mat.recyclable_percentage,
                lithium_content_g: mat.lithium_content_g,
                cobalt_content_g: mat.cobalt_content_g,
                nickel_content_g: mat.nickel_content_g,
                manganese_content_g: mat.manganese_content_g,
                lead_content_g: mat.lead_content_g,
                cadmium_content_g: mat.cadmium_content_g,
                hazardous_substances: mat.hazardous_substances,
                supply_chain_source: mat.supply_chain_source,
            };

            let material_service = MaterialService::new(self.engine.encryption.clone());
            let (row, data_hash) = material_service
                .prepare_submission(&domain_material)
                .map_err(|e| Status::internal(format!("material encryption failed: {}", e)))?;

            let repo = MaterialRepositoryImpl::new(self.engine.db_pool.clone());
            use crate::repositories::material_repo::MaterialRepository;
            repo.insert(&row, manufacturer_uuid, &data_hash)
                .await
                .map_err(|e| Status::internal(format!("material insert failed: {}", e)))?;
        }

        info!(
            bpan = %reg_response.bpan,
            manufacturer = %req.manufacturer_id,
            "Battery registered successfully via gRPC"
        );

        Ok(Response::new(RegisterBatteryResponse {
            bpan: reg_response.bpan,
            qr_code_png: vec![],
            qr_payload: String::new(),
        }))
    }

    async fn get_battery(
        &self,
        request: Request<GetBatteryRequest>,
    ) -> Result<Response<GetBatteryResponse>, Status> {
        let req = request.into_inner();

        // Fetch battery descriptor from DB
        let row = sqlx::query(
            r#"
            SELECT chemistry_type, nominal_voltage, rated_capacity_kwh,
                   energy_density, weight_kg, form_factor
            FROM battery_descriptor WHERE bpan = $1
            "#,
        )
        .bind(&req.bpan)
        .fetch_optional(&self.engine.db_pool)
        .await
        .map_err(|e| Status::internal(format!("db query failed: {}", e)))?
        .ok_or_else(|| Status::not_found(format!("battery {} not found", req.bpan)))?;

        let chemistry_type: String = row.get("chemistry_type");
        let nominal_voltage: f64 = row.get("nominal_voltage");
        let capacity_kwh: f64 = row.get("rated_capacity_kwh");
        let weight_kg: f64 = row.get("weight_kg");

        // Fetch dynamic data (health status)
        let health_row = sqlx::query(
            r#"
            SELECT state_of_health, total_cycles, degradation_class, end_of_life
            FROM battery_health WHERE bpan = $1 ORDER BY updated_at DESC LIMIT 1
            "#,
        )
        .bind(&req.bpan)
        .fetch_optional(&self.engine.db_pool)
        .await
        .map_err(|e| Status::internal(format!("db query failed: {}", e)))?;

        let (soh, status) = if let Some(h) = health_row {
            let soh: f64 = h.get("state_of_health");
            let eol: bool = h.get("end_of_life");
            let status = if eol {
                BatteryStatus::EndOfLife
            } else if soh > 80.0 {
                BatteryStatus::Operational
            } else if soh >= 60.0 {
                BatteryStatus::SecondLife
            } else {
                BatteryStatus::Waste
            };
            (soh as f32, status)
        } else {
            (100.0, BatteryStatus::Operational)
        };

        let static_data = BatteryStaticData {
            country_code: String::new(),      // Encrypted in battery_identifiers
            manufacturer_code: String::new(), // Encrypted in battery_identifiers
            battery_capacity_kwh: capacity_kwh as f32,
            battery_chemistry: chemistry_type,
            nominal_voltage: nominal_voltage as f32,
            cell_origin: String::new(),
            extinguisher_class: String::new(),
            manufacturing_date: String::new(),
            factory_code: String::new(),
            sequential_number: String::new(),
            tac_number: String::new(),
            num_cells: 0,
            internal_resistance_ohm: 0.0,
            weight_kg: weight_kg as f32,
            warranty_years: 0,
            cell_type: String::new(),
            total_carbon_footprint_kgco2e_per_kwh: 0.0,
        };

        Ok(Response::new(GetBatteryResponse {
            bpan: req.bpan,
            static_data: Some(static_data),
        }))
    }

    async fn update_battery_status(
        &self,
        request: Request<UpdateBatteryStatusRequest>,
    ) -> Result<Response<UpdateBatteryStatusResponse>, Status> {
        let req = request.into_inner();

        // Update health record
        sqlx::query(
            r#"
            UPDATE battery_health SET
                state_of_health = $1,
                degradation_class = $2,
                updated_at = NOW()
            WHERE bpan = $3
            "#,
        )
        .bind(req.state_of_health as f64)
        .bind("normal")
        .bind(&req.bpan)
        .execute(&self.engine.db_pool)
        .await
        .map_err(|e| Status::internal(format!("db update failed: {}", e)))?;

        info!(
            bpan = %req.bpan,
            soh = req.state_of_health,
            "Battery health updated via gRPC"
        );

        let new_status = match req.new_status {
            1 => "OPERATIONAL", // BatteryStatus::Operational
            2 => "SECOND_LIFE", // BatteryStatus::SecondLife
            3 => "END_OF_LIFE", // BatteryStatus::EndOfLife
            4 => "WASTE",       // BatteryStatus::Waste
            _ => "UNKNOWN",
        };

        Ok(Response::new(UpdateBatteryStatusResponse {
            success: true,
            new_status: new_status.to_string(),
        }))
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

        let (row, data_hash) = self
            .engine
            .material_service
            .prepare_submission(&comp)
            .map_err(|e| Status::internal(format!("encryption failed: {}", e)))?;

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

/// Derive battery category from chemistry type and capacity.
/// Maps to the spec's BatteryCategory enum values.
fn derive_battery_category(chemistry: &str, capacity_kwh: f32) -> String {
    let chemistry_upper = chemistry.to_uppercase();
    let is_ev_chemistry = matches!(
        chemistry_upper.as_str(),
        "NMC" | "NCA" | "LFP" | "SOLID-STATE"
    );

    if is_ev_chemistry && capacity_kwh < 2.0 {
        "EV-L".to_string()
    } else if is_ev_chemistry {
        "EV-M".to_string()
    } else {
        "INDUSTRIAL".to_string()
    }
}
