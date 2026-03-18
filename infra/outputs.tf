output "server_ip" {
  description = "Hookbot server public IP"
  value       = google_compute_address.hookbot.address
}

output "server_url" {
  description = "Hookbot server URL"
  value       = "http://${google_compute_address.hookbot.address}"
}

output "cloudflare_pages_url" {
  description = "Cloudflare Pages frontend URL"
  value       = "https://${cloudflare_pages_project.hookbot_web.subdomain}"
}

output "artifact_registry" {
  description = "Docker image registry path"
  value       = "${var.gcp_region}-docker.pkg.dev/${var.gcp_project_id}/hookbot/server"
}
