package models

type RegisterStakeholderPayload struct {
	Email                string `json:"email" example:"admin@example.com"`
	Password             string `json:"password" example:"MySecureP@ssw0rd!"`
	Role                 string `json:"role" example:"Government Authority"`
	ProfileDetails       string `json:"profileDetails" example:"{}"`
	AadharNumber         string `json:"aadharNumber" example:"123456789012"`
	AadharDocumentBase64 string `json:"aadharDocumentBase64" example:"data:image/png;base64,iVBORw0KGgo..."`
}

type RegisterStakeholderResponseJSON struct {
	StakeholderID string `json:"stakeholderId" example:"123e4567-e89b-12d3-a456-426614174000"`
	Status        string `json:"status" example:"SUCCESS"`
}

type LoginPayload struct {
	Email    string `json:"email" example:"admin@example.com"`
	Password string `json:"password" example:"MySecureP@ssw0rd!"`
}

type LoginResponseJSON struct {
	StakeholderID string `json:"stakeholderId" example:"123e4567-e89b-12d3-a456-426614174000"`
	Role          string `json:"role" example:"Government Authority"`
	Message       string `json:"message" example:"Logged in successfully"`
}

type RefreshResponseJSON struct {
	Message string `json:"message" example:"Token refreshed successfully"`
}
