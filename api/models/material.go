// material.go — Request/response models for BMCS (Battery Material Composition Sheet)
// Maps to proto MaterialComposition message and enforces role-based field visibility.

package models

// MaterialCompositionRequest is the JSON body for POST /batteries/{bpan}/material
type MaterialCompositionRequest struct {
	// Public fields
	CathodeMaterial    string  `json:"cathode_material" example:"NMC811"`
	AnodeMaterial      string  `json:"anode_material" example:"Graphite"`
	ElectrolyteType    string  `json:"electrolyte_type" example:"LiPF6"`
	SeparatorMaterial  string  `json:"separator_material" example:"PE/PP"`
	RecyclablePercent  float64 `json:"recyclable_percentage" example:"92.5"`

	// Private fields (encrypted at Rust layer)
	LithiumContentG    float64 `json:"lithium_content_g" example:"450.0"`
	CobaltContentG     float64 `json:"cobalt_content_g" example:"120.0"`
	NickelContentG     float64 `json:"nickel_content_g" example:"310.0"`
	ManganeseContentG  float64 `json:"manganese_content_g" example:"85.0"`
	LeadContentG       float64 `json:"lead_content_g" example:"0.0"`
	CadmiumContentG    float64 `json:"cadmium_content_g" example:"0.0"`
	HazardousSubstances string `json:"hazardous_substances" example:"LiPF6"`
	SupplyChainSource  string  `json:"supply_chain_source" example:"Korea/Posco"`
}

// MaterialCompositionResponse is the JSON body returned from GET /batteries/{bpan}/material
type MaterialCompositionResponse struct {
	BPAN               string  `json:"bpan"`
	CathodeMaterial    string  `json:"cathode_material"`
	AnodeMaterial      string  `json:"anode_material"`
	ElectrolyteType    string  `json:"electrolyte_type"`
	SeparatorMaterial  string  `json:"separator_material"`
	RecyclablePercent  float64 `json:"recyclable_percentage"`

	// Private fields — populated only for authorised roles
	LithiumContentG    float64 `json:"lithium_content_g,omitempty"`
	CobaltContentG     float64 `json:"cobalt_content_g,omitempty"`
	NickelContentG     float64 `json:"nickel_content_g,omitempty"`
	ManganeseContentG  float64 `json:"manganese_content_g,omitempty"`
	LeadContentG       float64 `json:"lead_content_g,omitempty"`
	CadmiumContentG    float64 `json:"cadmium_content_g,omitempty"`
	HazardousSubstances string `json:"hazardous_substances,omitempty"`
	SupplyChainSource  string  `json:"supply_chain_source,omitempty"`

	Partial bool `json:"partial"` // true if private fields were redacted
}

// SubmitMaterialResponse is the JSON body for successful POST
type SubmitMaterialResponse struct {
	Success   bool   `json:"success"`
	DataHash  string `json:"data_hash"`
	EventHash string `json:"event_hash"`
}
