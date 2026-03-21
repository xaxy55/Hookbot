# ============================================================
#  Hookbot Makefile
# ============================================================
#  Colors
# ============================================================
BOLD   := \033[1m
RESET  := \033[0m
CYAN   := \033[36m
GREEN  := \033[32m
YELLOW := \033[33m
BLUE   := \033[34m
MAGENTA:= \033[35m
RED    := \033[31m
DIM    := \033[2m

.PHONY: help \
        test \
        server web up build \
        lint lint-fix lint-server lint-web lint-ios lint-fix-ios swift-check \
        update update-server update-web \
        build-testflight screenshots \
        gh-secrets cloud-secrets \
        cli-build cli-security cli-config cli-status cli-doctor cli-ping

# ============================================================
#  Default target
# ============================================================
help: ## Show this help
	@printf "\n$(BOLD)$(CYAN) Hookbot$(RESET)\n\n"
	@printf "$(DIM) Usage: make <target>$(RESET)\n\n"
	@printf "$(BOLD)$(YELLOW) Development$(RESET)\n"
	@grep -E '^(server|web|up):.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@printf "\n$(BOLD)$(YELLOW) Build$(RESET)\n"
	@grep -E '^(build|build-testflight|screenshots):.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@printf "\n$(BOLD)$(YELLOW) Testing$(RESET)\n"
	@grep -E '^test:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@printf "\n$(BOLD)$(YELLOW) Linting$(RESET)\n"
	@grep -E '^(lint|swift).*:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@printf "\n$(BOLD)$(YELLOW) Updates$(RESET)\n"
	@grep -E '^update.*:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@printf "\n$(BOLD)$(YELLOW) CLI$(RESET)\n"
	@grep -E '^cli.*:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@printf "\n$(BOLD)$(YELLOW) Secrets & CI$(RESET)\n"
	@grep -E '^(gh-secrets|cloud-secrets):.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@printf "\n"

# ============================================================
#  Development
# ============================================================
ADMIN_PASSWORD ?= $(shell grep '^ADMIN_PASSWORD=' .env 2>/dev/null | cut -d= -f2)

server: ## Start backend dev server (port 3000, debug logging)
	@printf "$(GREEN)>> Starting Rust server...$(RESET)\n"
	cd server && RUST_LOG=debug ADMIN_PASSWORD=$(ADMIN_PASSWORD) cargo run

web: ## Start frontend dev server (port 5173)
	@printf "$(GREEN)>> Starting Vite dev server...$(RESET)\n"
	cd web && npm run dev

up: ## Start everything with Docker Compose
	@printf "$(GREEN)>> Starting Docker Compose stack...$(RESET)\n"
	docker compose up --build

# ============================================================
#  Build
# ============================================================
build: ## Build server and web for production
	@printf "$(BLUE)>> Building server (release)...$(RESET)\n"
	cd server && cargo build --release
	@printf "$(BLUE)>> Building web...$(RESET)\n"
	cd web && npm run build
	@printf "$(GREEN)>> Build complete.$(RESET)\n"

build-testflight: ## Archive and upload iOS, Mac, and watchOS to TestFlight
	@printf "$(BLUE)>> Building for TestFlight...$(RESET)\n"
	cd ios && ./build-testflight.sh

SCREENSHOT_SIM ?= iPhone 17 Pro Max

screenshots: ## Generate App Store screenshots via UI tests
	@printf "$(BLUE)>> Generating screenshots on '$(SCREENSHOT_SIM)'...$(RESET)\n"
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
	@printf "\n$(GREEN)>> Screenshots saved to ios/screenshots/$(RESET)\n"
	@ls -la ios/screenshots/*.png 2>/dev/null || true

# ============================================================
#  Testing
# ============================================================
test: ## Run Playwright tests
	@printf "$(MAGENTA)>> Running Playwright tests...$(RESET)\n"
	npx playwright test

# ============================================================
#  Linting
# ============================================================
lint: lint-server lint-web lint-ios ## Lint server, web, and iOS

lint-server: ## Lint Rust server with Clippy
	@printf "$(YELLOW)>> Clippy (server)...$(RESET)\n"
	cd server && cargo clippy --all-targets --all-features -- -D warnings

lint-web: ## Lint web with ESLint
	@printf "$(YELLOW)>> ESLint (web)...$(RESET)\n"
	cd web && npm run lint

lint-ios: ## Lint Swift code with SwiftLint
	@printf "$(YELLOW)>> SwiftLint (iOS)...$(RESET)\n"
	cd ios && swiftlint lint

lint-fix-ios: ## Auto-fix Swift lint issues (SwiftLint)
	@printf "$(YELLOW)>> SwiftLint --fix (iOS)...$(RESET)\n"
	cd ios && swiftlint lint --fix && swiftlint lint

swift-check: ## Syntax check iOS project (xcodebuild build, no codesign)
	@printf "$(YELLOW)>> Swift syntax check (xcodebuild)...$(RESET)\n"
	cd ios && xcodebuild build \
		-project Hookbot.xcodeproj \
		-scheme Hookbot \
		-destination 'generic/platform=iOS' \
		CODE_SIGN_IDENTITY=- \
		CODE_SIGNING_REQUIRED=NO \
		CODE_SIGNING_ALLOWED=NO \
		-quiet

lint-fix: lint-fix-server lint-fix-web lint-fix-ios ## Auto-fix lint issues in server, web, and iOS

lint-fix-server: ## Auto-fix Rust lint issues (cargo fix + fmt)
	@printf "$(YELLOW)>> cargo fix + fmt (server)...$(RESET)\n"
	cd server && cargo fix --allow-dirty --allow-staged
	cd server && cargo fmt

lint-fix-web: ## Auto-fix web lint issues (eslint --fix)
	@printf "$(YELLOW)>> ESLint --fix (web)...$(RESET)\n"
	cd web && npx eslint . --fix

# ============================================================
#  Updates
# ============================================================
update: update-server update-web ## Update all dependencies

update-server: ## Update Rust dependencies (cargo update)
	@printf "$(CYAN)>> Updating Rust dependencies...$(RESET)\n"
	cd server && cargo update

update-web: ## Update npm dependencies (npm update)
	@printf "$(CYAN)>> Updating npm dependencies (web)...$(RESET)\n"
	cd web && npm update

# ============================================================
#  CLI
# ============================================================
cli-build: ## Build hookbot CLI tool
	@printf "$(BLUE)>> Building hookbot CLI...$(RESET)\n"
	cd cli && cargo build --release
	@printf "$(GREEN)>> CLI built: cli/target/release/hookbot$(RESET)\n"

cli-security: cli-build ## Run OWASP security audit against live instance
	./cli/target/release/hookbot security --target https://bot.mr-ai.no --frontend https://hookbot.mr-ai.no

cli-config: cli-build ## Validate local .env configuration
	./cli/target/release/hookbot config

cli-status: cli-build ## Check server health and device status
	./cli/target/release/hookbot --url https://bot.mr-ai.no status

cli-doctor: cli-build ## Full diagnostic (config + security + connectivity)
	./cli/target/release/hookbot --url https://bot.mr-ai.no doctor

cli-ping: cli-build ## Ping server to check connectivity
	./cli/target/release/hookbot --url https://bot.mr-ai.no ping

# ============================================================
#  Secrets & CI
# ============================================================
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
		printf "$(GREEN)>> GitHub secrets set successfully.$(RESET)\n"

cloud-secrets: ## Set GitHub secrets for GCE + Cloudflare deployment (interactive)
	@printf "$(CYAN)>> === Google Cloud (Workload Identity Federation) ===\n$(RESET)"
	@read -p "GCP Project ID: " gcp_proj; \
	read -p "WIF Provider (projects/PROJECT_NUM/locations/global/workloadIdentityPools/POOL/providers/PROVIDER): " wif_provider; \
	read -p "Service Account Email (e.g. hookbot-deploy@PROJECT.iam.gserviceaccount.com): " sa_email; \
	printf "\n$(CYAN)>> === Cloudflare ===\n$(RESET)"; \
	read -p "Cloudflare API Token: " cf_token; \
	read -p "Cloudflare Account ID: " cf_account; \
	printf "\n$(CYAN)>> === API URL ===\n$(RESET)"; \
	printf "$(DIM)(This is your GCE static IP — run 'terraform output server_ip' after first deploy)$(RESET)\n"; \
	read -p "API Base URL (e.g. http://34.xx.xx.xx): " api_url; \
	printf "\n$(YELLOW)>> Setting GitHub secrets...$(RESET)\n"; \
	gh secret set GCP_PROJECT_ID --body "$$gcp_proj" && \
	gh secret set GCP_WIF_PROVIDER --body "$$wif_provider" && \
	gh secret set GCP_SA_EMAIL --body "$$sa_email" && \
	gh secret set CLOUDFLARE_API_TOKEN --body "$$cf_token" && \
	gh secret set CLOUDFLARE_ACCOUNT_ID --body "$$cf_account" && \
	gh secret set API_BASE_URL --body "$$api_url" && \
	printf "\n$(GREEN)>> All cloud secrets set successfully!$(RESET)\n"
