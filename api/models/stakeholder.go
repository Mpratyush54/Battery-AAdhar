// stakeholder.go — Stakeholder model (manufacturer, recycler, government, etc.)

package models

import (
	"time"
	"github.com/google/uuid"
)

// StakeholderType represents the type of stakeholder
type StakeholderType string

const (
	TypeManufacturer    StakeholderType = "manufacturer"
	TypeImporter        StakeholderType = "importer"
	TypeDistributor     StakeholderType = "distributor"
	TypeServiceProvider StakeholderType = "service_provider"
	TypeRecycler        StakeholderType = "recycler"
	TypeReuseOperator   StakeholderType = "reuse_operator"
	TypeGovernment      StakeholderType = "government"
	TypeOEM             StakeholderType = "oem"
	TypeFinancier       StakeholderType = "financier"
)

// Stakeholder represents a single entity in the BPA ecosystem
type Stakeholder struct {
	ID                uuid.UUID
	StakeholderType   StakeholderType
	OrganizationName  string
	CountryCode       string // 2-letter ISO code
	RegulationCode    string // Manufacturer code (BMI), if applicable
	ContactEmail      string
	ContactPhone      string
	AadhaarEncrypted  []byte // KYC document (encrypted by Rust service)
	PublicKey         []byte // Ed25519 public key (for verification)
	Status            string // "active", "pending", "suspended"
	CreatedAt         time.Time
	UpdatedAt         time.Time
}

// StakeholderRegistration is the request to register a new stakeholder
type StakeholderRegistration struct {
	StakeholderType  StakeholderType `json:"stakeholder_type"`
	OrganizationName string          `json:"organization_name"`
	CountryCode      string          `json:"country_code"`
	ContactEmail     string          `json:"contact_email"`
	ContactPhone     string          `json:"contact_phone"`
	AadhaarDocument  []byte          `json:"aadhaar_document"` // Raw KYC doc (will be encrypted)
}

// StakeholderResponse is the JSON response
type StakeholderResponse struct {
	ID               string    `json:"id"`
	StakeholderType  string    `json:"stakeholder_type"`
	OrganizationName string    `json:"organization_name"`
	CountryCode      string    `json:"country_code"`
	Status           string    `json:"status"`
	CreatedAt        time.Time `json:"created_at"`
}
