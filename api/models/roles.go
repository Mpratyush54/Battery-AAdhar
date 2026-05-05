// roles.go — Stakeholder roles and permissions
// Based on BPA spec Table 1 (List of Stakeholders)

package models

// StakeholderRole defines a role in the BPA system
type StakeholderRole string

const (
	// BPA stakeholder roles (from spec Table 1)
	RoleManufacturer    StakeholderRole = "manufacturer"
	RoleImporter        StakeholderRole = "importer"
	RoleDistributor     StakeholderRole = "distributor"
	RoleServiceProvider StakeholderRole = "service_provider"
	RoleRecycler        StakeholderRole = "recycler"
	RoleReuseOperator   StakeholderRole = "reuse_operator"
	RoleGovernment      StakeholderRole = "government"
	RoleAdmin           StakeholderRole = "admin"
	RolePublic          StakeholderRole = "public"
	RoleAuthenticated   StakeholderRole = "authenticated"
)

// Resource defines what can be accessed
type Resource string

const (
	ResourceBattery              Resource = "battery"
	ResourceBatteryMaterial      Resource = "battery_material"
	ResourceBatteryHealth        Resource = "battery_health"
	ResourceBatteryCertification Resource = "battery_certification"
	ResourceBatteryLifecycle     Resource = "battery_lifecycle"
	ResourceManufacturer         Resource = "manufacturer"
	ResourceAuditLog             Resource = "audit_log"
)

// Action defines what can be done
type Action string

const (
	ActionCreate   Action = "create"
	ActionRead     Action = "read"
	ActionUpdate   Action = "update"
	ActionDelete   Action = "delete"
	ActionApprove  Action = "approve"
	ActionRecycle  Action = "recycle"
	ActionTransfer Action = "transfer"
	ActionVerify   Action = "verify"
)

// Permission represents a single role+resource+action grant
type Permission struct {
	Role     StakeholderRole
	Resource Resource
	Action   Action
}

// AccessMatrix defines all role→resource→action permissions
// Based on spec Table 2 (Data Access Control Matrix)
var AccessMatrix = map[StakeholderRole]map[Resource][]Action{
	// MANUFACTURER: Create batteries, view own, update status
	RoleManufacturer: {
		ResourceBattery:         {ActionCreate, ActionRead, ActionUpdate},
		ResourceBatteryMaterial: {ActionCreate, ActionUpdate},
		ResourceManufacturer:    {ActionRead, ActionUpdate},
	},

	// IMPORTER: View battery static data, import to country
	RoleImporter: {
		ResourceBattery: {ActionRead},
	},

	// DISTRIBUTOR: View battery data, transfer ownership
	RoleDistributor: {
		ResourceBattery:          {ActionRead, ActionTransfer},
		ResourceBatteryLifecycle: {ActionRead, ActionUpdate},
	},

	// SERVICE PROVIDER: View all data (no private), update health
	RoleServiceProvider: {
		ResourceBattery:              {ActionRead, ActionUpdate},
		ResourceBatteryHealth:        {ActionRead, ActionUpdate},
		ResourceBatteryCertification: {ActionRead},
	},

	// RECYCLER: View lifecycle, record recycling
	RoleRecycler: {
		ResourceBatteryLifecycle: {ActionRead, ActionUpdate},
		ResourceBattery:          {ActionRead},
	},

	// REUSE OPERATOR: Certify second-life, transfer ownership
	RoleReuseOperator: {
		ResourceBatteryLifecycle: {ActionRead, ActionUpdate},
		ResourceBattery:          {ActionRead},
	},

	// GOVERNMENT: Read-only access to all (audit + verify)
	RoleGovernment: {
		ResourceBattery:              {ActionRead},
		ResourceBatteryMaterial:      {ActionRead},
		ResourceBatteryHealth:        {ActionRead},
		ResourceBatteryCertification: {ActionRead},
		ResourceBatteryLifecycle:     {ActionRead},
		ResourceAuditLog:             {ActionRead},
	},

	// ADMIN: Full access
	RoleAdmin: {
		ResourceBattery:              {ActionCreate, ActionRead, ActionUpdate, ActionDelete, ActionApprove},
		ResourceBatteryMaterial:      {ActionCreate, ActionRead, ActionUpdate, ActionDelete},
		ResourceBatteryHealth:        {ActionCreate, ActionRead, ActionUpdate, ActionDelete},
		ResourceBatteryCertification: {ActionCreate, ActionRead, ActionUpdate, ActionDelete},
		ResourceBatteryLifecycle:     {ActionCreate, ActionRead, ActionUpdate, ActionDelete},
		ResourceManufacturer:         {ActionCreate, ActionRead, ActionUpdate, ActionDelete},
		ResourceAuditLog:             {ActionRead},
	},

	// PUBLIC: Read-only public battery data
	RolePublic: {
		ResourceBattery: {ActionRead},
	},

	// AUTHENTICATED: Same as public for now (login required)
	RoleAuthenticated: {
		ResourceBattery: {ActionRead},
	},
}

// CanAccess checks if a role can perform an action on a resource
func CanAccess(role StakeholderRole, resource Resource, action Action) bool {
	resourceMap, roleExists := AccessMatrix[role]
	if !roleExists {
		return false
	}

	actions, resourceExists := resourceMap[resource]
	if !resourceExists {
		return false
	}

	for _, a := range actions {
		if a == action {
			return true
		}
	}

	return false
}
