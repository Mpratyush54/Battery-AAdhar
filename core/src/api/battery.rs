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
            .ok_or_else(|| {
                Status::not_found(format!("no BMCS found for BPAN {}", req.bpan))
            })?;

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
