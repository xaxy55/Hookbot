resource "cloudflare_pages_project" "hookbot_web" "vite_api_base_url" {
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
        VITE_API_BASE_URL = var.vite_api_base_url
        NODE_VERSION      = "20"
      }
    }

    preview {
      environment_variables = {
        VITE_API_BASE_URL = var.vite_api_base_url
        NODE_VERSION      = "20"
      }
    }
  }
}
