terraform {
  required_version = ">= 1.5"

  backend "gcs" {
    bucket = "hookbot-tfstate"
    prefix = "terraform/state"
  }

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 4.0"
    }
  }
}

provider "google" {
  project = var.gcp_project_id
  region  = var.gcp_region
  zone    = var.gcp_zone
  workos_domain = var.workos_domain
}

provider "cloudflare" {
  api_token = var.cloudflare_api_token
  vite_api_base_url = var.vite_api_base_url
}
