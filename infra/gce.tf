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
            { name = "BIND_ADDR", value = "0.0.0.0:3000" },
          ]
          volumeMounts = [{
            name      = "hookbot-data"
            mountPath = "/app/data"
          }]
        }]
        volumes = [{
          name = "hookbot-data"
          hostPath = {
            path = "/mnt/disks/hookbot-data"
          }
        }]
        restartPolicy = "Always"
      }
    })

    startup-script = <<-EOF
      #!/bin/bash
      mkdir -p /mnt/disks/hookbot-data/firmware
    EOF
  }
}
