.PHONY: help test server web up build build-testflight screenshots gh-secrets cloud-secrets

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

test: ## Run Playwright tests
	npx playwright test

server: ## Start backend dev server (port 3000, debug logging)
	ADMIN_PASSWORD ?= $(shell grep '^ADMIN_PASSWORD=' .env 2>/dev/null | cut -d= -f2)
	cd server && RUST_LOG=debug ADMIN_PASSWORD=$(ADMIN_PASSWORD) cargo run

web: ## Start frontend dev server (port 5173)
	cd web && npm run dev

build: ## Build server and web for production
	cd server && cargo build --release
	cd web && npm run build

gh-secrets: ## Set GitHub secrets for Docker Hub from .env (prompts if missing)
	@if [ ! -f .env ]; then \
		read -p "DOCKERHUB_USERNAME: " dhu; \
		read -p "DOCKERHUB_TOKEN: " dht; \
		echo "DOCKERHUB_USERNAME=$$dhu" > .env; \
		echo "DOCKERHUB_TOKEN=$$dht" >> .env; \
		echo "Saved to .env"; \
	fi
	@. ./.env && \
		gh auth switch --user xaxy55 && \
		gh secret set DOCKERHUB_USERNAME --body "$$DOCKERHUB_USERNAME" && \
		gh secret set DOCKERHUB_TOKEN --body "$$DOCKERHUB_TOKEN" && \
		echo "GitHub secrets set successfully"

cloud-secrets: ## Set GitHub secrets for GCE + Cloudflare deployment (interactive)
	@echo "=== Google Cloud (Workload Identity Federation) ==="
	@read -p "GCP Project ID: " gcp_proj; \
	read -p "WIF Provider (projects/PROJECT_NUM/locations/global/workloadIdentityPools/POOL/providers/PROVIDER): " wif_provider; \
	read -p "Service Account Email (e.g. hookbot-deploy@PROJECT.iam.gserviceaccount.com): " sa_email; \
	echo ""; \
	echo "=== Cloudflare ==="; \
	read -p "Cloudflare API Token: " cf_token; \
	read -p "Cloudflare Account ID: " cf_account; \
	echo ""; \
	echo "=== API URL ==="; \
	echo "(This is your GCE static IP — run 'terraform output server_ip' after first deploy)"; \
	read -p "API Base URL (e.g. http://34.xx.xx.xx): " api_url; \
	echo ""; \
	echo "Setting GitHub secrets..."; \
	gh secret set GCP_PROJECT_ID --body "$$gcp_proj" && \
	gh secret set GCP_WIF_PROVIDER --body "$$wif_provider" && \
	gh secret set GCP_SA_EMAIL --body "$$sa_email" && \
	gh secret set CLOUDFLARE_API_TOKEN --body "$$cf_token" && \
	gh secret set CLOUDFLARE_ACCOUNT_ID --body "$$cf_account" && \
	gh secret set API_BASE_URL --body "$$api_url" && \
	echo "" && \
	echo "All cloud secrets set successfully!"

up: ## Start everything with Docker Compose
	docker compose up --build

build-testflight: ## Archive and upload iOS, Mac, and watchOS to TestFlight
	cd ios && ./build-testflight.sh

SCREENSHOT_SIM ?= iPhone 17 Pro Max

screenshots: ## Generate App Store screenshots via UI tests
	cd ios && xcodegen generate
	xcrun simctl boot "$(SCREENSHOT_SIM)" 2>/dev/null || true
	rm -rf ios/build/screenshots.xcresult
	cd ios && xcodebuild test \
		-project Hookbot.xcodeproj \
		-scheme Hookbot \
		-destination 'platform=iOS Simulator,name=$(SCREENSHOT_SIM)' \
		-only-testing:HookbotUITests/ScreenshotTests \
		-resultBundlePath build/screenshots.xcresult \
		CODE_SIGN_IDENTITY=- \
		CODE_SIGNING_REQUIRED=NO \
		-quiet
	mkdir -p ios/screenshots
	python3 scripts/extract-screenshots.py ios/build/screenshots.xcresult ios/screenshots/
	@echo ""
	@echo "Screenshots saved to ios/screenshots/"
	@ls -la ios/screenshots/*.png 2>/dev/null || true
