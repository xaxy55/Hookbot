resource "cloudflare_pages_project" "hookbot_web" {
  account_id        = var.cloudflare_account_id
  name              = var.cloudflare_pages_project
  production_branch = "main"

  build_config {
    build_command   = "npm run build"
    destination_dir = "dist"
    root_dir        = "web"
  }

  deployment_configs {
    production {
      environment_variables = {
        VITE_API_BASE_URL = "http://${google_compute_address.hookbot.address}"
        NODE_VERSION      = "20"
      }
    }

    preview {
      environment_variables = {
        VITE_API_BASE_URL = "http://${google_compute_address.hookbot.address}"
        NODE_VERSION      = "20"
      }
    }
  }
}
