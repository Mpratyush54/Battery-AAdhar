package models

import "time"

type ComplianceViolation struct {
	ViolationType  string     `json:"violation_type"`
	Severity       string     `json:"severity"` // "INFO", "WARNING", "CRITICAL"
	Description    string     `json:"description"`
	RequiresAction bool       `json:"requires_action"`
	ActionDeadline *time.Time `json:"action_deadline,omitempty"`
	DetectedAt     time.Time  `json:"detected_at"`
}

type ComplianceStatusResponse struct {
	BPAN          string                `json:"bpan"`
	Status        string                `json:"status"` // "COMPLIANT", "WARNINGS_EXIST", "VIOLATIONS_EXIST"
	Violations    []ComplianceViolation `json:"violations"`
	CriticalCount uint32                `json:"critical_count"`
	WarningCount  uint32                `json:"warning_count"`
	LastCheckedAt time.Time             `json:"last_checked_at"`
}

type ComplianceProofResponse struct {
	BPAN        string `json:"bpan"`
	Requirement string `json:"requirement"` // "operational", "second_life", "recyclable"
	Statement   string `json:"statement"`   // Human-readable claim
	Proof       []byte `json:"proof"`
	Commitment  []byte `json:"commitment"`
	Note        string `json:"note,omitempty"`
}

type ComplianceDashboard struct {
	TotalBatteries           uint32            `json:"total_batteries"`
	BatteriesWithViolations  uint32            `json:"batteries_with_violations"`
	CriticalViolations       uint32            `json:"critical_violations"`
	WarningViolations        uint32            `json:"warning_violations"`
	ComplianceRate           float32           `json:"compliance_rate_percent"`
	ViolationsByType         map[string]uint32 `json:"violations_by_type"`
	ViolationsByManufacturer map[string]uint32 `json:"violations_by_manufacturer"`
	LastScanAt               *time.Time        `json:"last_scan_at,omitempty"`
}
