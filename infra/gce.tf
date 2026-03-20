resource "google_compute_address" "hookbot" {
  name = "hookbot-ip"
}

resource "google_compute_firewall" "hookbot_http" {
  name    = "hookbot-allow-http"
  network = "default"

  allow {
    protocol = "tcp"
    ports    = ["80", "443"]
  }

  source_ranges = ["0.0.0.0/0"]
  target_tags   = ["hookbot-server"]
}

resource "google_artifact_registry_repository" "hookbot" {
  location      = var.gcp_region
  repository_id = "hookbot"
  format        = "DOCKER"
}

resource "google_service_account" "hookbot_vm" {
  account_id   = "hookbot-vm"
  display_name = "Hookbot VM"
}

resource "google_project_iam_member" "hookbot_ar_reader" {
  project = var.gcp_project_id
  role    = "roles/artifactregistry.reader"
  member  = "serviceAccount:${google_service_account.hookbot_vm.email}"
}

resource "google_compute_instance" "hookbot" {
  name         = "hookbot-server"
  machine_type = "e2-micro"
  zone         = var.gcp_zone
  tags         = ["hookbot-server"]

  boot_disk {
    initialize_params {
      image = "projects/cos-cloud/global/images/family/cos-stable"
      size  = 30
      type  = "pd-standard"
    }
  }

  network_interface {
    network = "default"
    access_config {
      nat_ip = google_compute_address.hookbot.address
    }
  }

  service_account {
    email  = google_service_account.hookbot_vm.email
    scopes = ["cloud-platform"]
  }

  metadata = {
    gce-container-declaration = yamlencode({
      spec = {
        containers = [{
          image = "${var.gcp_region}-docker.pkg.dev/${var.gcp_project_id}/hookbot/server:latest"
          env = [
            { name = "DATABASE_URL", value = "/app/data/hookbot.db" },
            { name = "FIRMWARE_DIR", value = "/app/data/firmware" },
            { name = "TLS_CERT_PATH", value = "/app/certs/origin.pem" },
            { name = "TLS_KEY_PATH", value = "/app/certs/origin-key.pem" },
            { name = "API_KEY", value = var.hookbot_api_key },
            { name = "ADMIN_PASSWORD", value = var.hookbot_admin_password },
            { name = "ALLOWED_ORIGINS", value = "https://bot.mr-ai.no,https://hookbot.mr-ai.no,https://hookbot-web.pages.dev" },
            { name = "WORKOS_CLIENT_ID", value = var.workos_client_id },
            { name = "WORKOS_API_KEY", value = var.workos_api_key },
            { name = "WORKOS_REDIRECT_URI", value = "https://hookbot.mr-ai.no/auth/callback" },
          ]
          volumeMounts = [{
            name      = "hookbot-data"
            mountPath = "/app/data"
          }]
        }]
        volumes = [{
          name = "hookbot-data"
          hostPath = {
            path = "/home/hookbot-data"
          }
        }]
        restartPolicy = "Always"
      }
    })

    startup-script = <<-EOF
      #!/bin/bash
      mkdir -p /home/hookbot-data/firmware
    EOF
  }
}
