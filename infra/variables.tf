variable "gcp_project_id" {
  description = "Google Cloud project ID"
  type        = string
}

variable "gcp_region" {
  description = "GCP region (must be us-west1, us-central1, or us-east1 for free tier)"
  type        = string
  default     = "us-central1"
}

variable "gcp_zone" {
  description = "GCP zone"
  type        = string
  default     = "us-central1-a"
}

variable "cloudflare_api_token" {
  description = "Cloudflare API token"
  type        = string
  sensitive   = true
}

variable "cloudflare_account_id" {
  description = "Cloudflare account ID"
  type        = string
}

variable "hookbot_api_key" {
  description = "API key for device/hook authentication (auto-generated if empty)"
  type        = string
  default     = ""
  sensitive   = true
}

variable "hookbot_admin_password" {
  description = "Admin password for web dashboard (auto-generated if empty)"
  type        = string
  default     = ""
  sensitive   = true
}

variable "workos_client_id" {
  description = "WorkOS Client ID for multi-tenant auth"
  type        = string
  default     = ""
}

variable "workos_api_key" {
  description = "WorkOS API key"
  type        = string
  default     = ""
  sensitive   = true
}

variable "cloudflare_pages_project" {
  description = "Cloudflare Pages project name"
  type        = string
  default     = "hookbot-web"
}

variable "vite_api_base_url" {
  description = "Vite api base url"
  type        = string
  default     = ""
}

variable "workos_domain" {
  description = "work os doamin url"
  type        = string
  default     = ""
}

variable "domain_name" {
  description = "Domain name"
  type        = string
  default     = ""
}

variable "frontend_sub_domain" {
  description = "Sub domain for the frontend"
  type        = string
  default     = ""
}