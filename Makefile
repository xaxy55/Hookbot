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
        test deploy release require-server-url \
        firmware firmware-c6 firmware-c6-upload \
        server web up build \
        lint lint-fix lint-server lint-web lint-ios lint-fix-ios swift-check \
        check-ios-project \
        update update-server update-web \
        build-testflight screenshots \
        gh-secrets cloud-secrets \
        install \
        cli-build cli-security cli-config cli-status cli-doctor cli-ping \
        security-audit audit-secrets audit-deps

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
	@printf "\n$(BOLD)$(YELLOW) Firmware$(RESET)\n"
	@grep -E '^firmware.*:.*?## .*$$' $(MAKEFILE_LIST) \
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
	@grep -E '^(install|cli.*):.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@printf "\n$(BOLD)$(YELLOW) Deploy$(RESET)\n"
	@grep -E '^deploy:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@printf "\n$(BOLD)$(YELLOW) Secrets & CI$(RESET)\n"
	@grep -E '^(gh-secrets|cloud-secrets):.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}'
	@printf "\n"

# ============================================================
#  Development
# ============================================================
# The C6 board is on the pioarduino platform, which conflicts with the official
# espressif32 platform over shared package names — keep it in its own store.
PIO_C6_CORE_DIR ?= $(HOME)/.platformio-pioarduino

ADMIN_PASSWORD ?= $(shell grep '^ADMIN_PASSWORD=' .env 2>/dev/null | cut -d= -f2)

# No environment-specific hostname lives in this repo — it is public. These come
# from .env (see .env.example), or pass them per invocation:
#   make cli-status HOOKBOT_SERVER_URL=https://your.server
env_val = $(shell grep '^$(1)=' .env 2>/dev/null | cut -d= -f2- | tr -d '"'"'"'"')
HOOKBOT_SERVER_URL   ?= $(call env_val,HOOKBOT_SERVER_URL)
HOOKBOT_FRONTEND_URL ?= $(call env_val,HOOKBOT_FRONTEND_URL)
DEPLOY_HOST          ?= $(call env_val,DEPLOY_HOST)
DEPLOY_DIR           ?= /opt/hookbot/deploy

# Fail loudly rather than silently running against an empty URL.
require-server-url:
	@if [ -z "$(HOOKBOT_SERVER_URL)" ]; then \
		printf "$(RED)>> HOOKBOT_SERVER_URL is not set.$(RESET)\n"; \
		printf "   Add it to .env or run: make <target> HOOKBOT_SERVER_URL=https://your.server\n"; \
		exit 1; \
	fi

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
#  Firmware
# ============================================================
firmware: ## Build firmware for the OLED and 4848 LCD boards
	cd firmware && pio run -e esp32 -e esp32-4848s040c

firmware-c6: ## Build firmware for the XIAO ESP32-C6 round LCD board
	cd firmware && PLATFORMIO_CORE_DIR=$(PIO_C6_CORE_DIR) pio run -e xiao-c6-gc9a01

# PlatformIO's port auto-detect can pick a Bluetooth serial device instead of
# the board ("Failed to connect to ESP32-C6: No serial data received"), so
# prefer the USB CDC port. Override with: make firmware-c6-upload PORT=/dev/...
PORT ?= $(shell ls /dev/cu.usbmodem* /dev/ttyACM* /dev/ttyUSB* 2>/dev/null | head -1)

firmware-c6-upload: ## Flash the XIAO ESP32-C6 round LCD board over USB
	cd firmware && PLATFORMIO_CORE_DIR=$(PIO_C6_CORE_DIR) pio run -e xiao-c6-gc9a01 --target upload $(if $(PORT),--upload-port $(PORT),)

# ============================================================
#  Testing
# ============================================================
# These tests run against a real device on the LAN. Point them at yours:
#   make test HOOKBOT_URL=http://192.168.1.50
HOOKBOT_URL ?= http://hookbot.local

test: ## Run Playwright tests against a device (override HOOKBOT_URL)
	@printf "$(MAGENTA)>> Running Playwright tests against $(HOOKBOT_URL)...$(RESET)\n"
	HOOKBOT_URL=$(HOOKBOT_URL) npx playwright test

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

lint-ios: check-ios-project ## Lint Swift code with SwiftLint
	@printf "$(YELLOW)>> SwiftLint (iOS)...$(RESET)\n"
	cd ios && swiftlint lint

check-ios-project: ## Verify every iOS source file is in the Xcode project
	@printf "$(YELLOW)>> Xcode project consistency...$(RESET)\n"
	@./scripts/check-ios-project.sh

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
# cargo install puts the binary in $(INSTALL_ROOT)/bin. Override to install
# elsewhere, e.g. make install INSTALL_ROOT=/usr/local
INSTALL_ROOT ?= $(HOME)/.cargo

install: ## Build and install the hookbot CLI onto your PATH
	@printf "$(BLUE)>> Installing hookbot CLI (release)...$(RESET)\n"
	cargo install --path cli --root $(INSTALL_ROOT) --locked --force
	@printf "$(GREEN)>> Installed: $(INSTALL_ROOT)/bin/hookbot$(RESET)\n"
	@$(INSTALL_ROOT)/bin/hookbot --version
	@case ":$$PATH:" in \
		*":$(INSTALL_ROOT)/bin:"*) ;; \
		*) printf "$(YELLOW)>> Add $(INSTALL_ROOT)/bin to your PATH to use 'hookbot'.$(RESET)\n" ;; \
	esac
	@printf "$(DIM)   Next: hookbot login --url <your-server> && hookbot hooks install$(RESET)\n"

cli-build: ## Build hookbot CLI tool
	@printf "$(BLUE)>> Building hookbot CLI...$(RESET)\n"
	cd cli && cargo build --release
	@printf "$(GREEN)>> CLI built: cli/target/release/hookbot$(RESET)\n"

cli-security: cli-build require-server-url ## Run OWASP security audit against live instance
	./cli/target/release/hookbot security --target $(HOOKBOT_SERVER_URL) $(if $(HOOKBOT_FRONTEND_URL),--frontend $(HOOKBOT_FRONTEND_URL),)

cli-config: cli-build ## Validate local .env configuration
	./cli/target/release/hookbot config

cli-status: cli-build require-server-url ## Check server health and device status
	./cli/target/release/hookbot --url $(HOOKBOT_SERVER_URL) status

cli-doctor: cli-build require-server-url ## Full diagnostic (config + security + connectivity)
	./cli/target/release/hookbot --url $(HOOKBOT_SERVER_URL) doctor

cli-ping: cli-build require-server-url ## Ping server to check connectivity
	./cli/target/release/hookbot --url $(HOOKBOT_SERVER_URL) ping

# ============================================================
#  Security
# ============================================================
# `cli-security` probes a *running* instance. These audit the source and the
# dependency tree, so they need no server and are safe to run in CI.
security-audit: audit-deps audit-secrets ## Audit dependencies and scan for credential leaks
	@printf "$(GREEN)>> Security audit complete.$(RESET)\n"

audit-secrets: ## Scan the tree for committed secrets and credentials leaked via the API
	@printf "$(YELLOW)>> Secret / credential-leak scan...$(RESET)\n"
	@./scripts/audit-secrets.sh

audit-deps: ## Check Rust and npm dependencies for known vulnerabilities
	@printf "$(YELLOW)>> cargo audit (server)...$(RESET)\n"
	@command -v cargo-audit >/dev/null 2>&1 || { \
		printf "$(DIM)   cargo-audit not installed; run: cargo install cargo-audit$(RESET)\n"; exit 1; }
	cd server && cargo audit
	@printf "$(YELLOW)>> cargo audit (cli)...$(RESET)\n"
	cd cli && cargo audit
	@printf "$(YELLOW)>> npm audit (web)...$(RESET)\n"
	@# --omit=dev: a vulnerability in a build-time tool is not shipped to a
	@# browser, and dev-only advisories drown out the ones that matter.
	cd web && npm audit --omit=dev --audit-level=high

# ============================================================
#  Deploy
# ============================================================
# The tag docker-push.yml publishes for a commit is its short SHA, so a deploy
# can name the exact image built from the code in front of you. Deploying
# ":latest" cannot do that: if the build for this commit never ran, ":latest"
# still resolves — to the previous commit — and every step reports success
# while shipping stale code.
GIT_SHA      = $(shell git rev-parse --short HEAD)
IMAGE_PREFIX ?= xaxyxy/hookbot
COMPOSE      = docker compose -f docker-compose.prod.yml --env-file .env
PINNED_IMAGES = SERVER_IMAGE=$(IMAGE_PREFIX)-server:$(GIT_SHA) \
                WEB_IMAGE=$(IMAGE_PREFIX)-web:$(GIT_SHA)

deploy: ## Deploy the image built from the current commit (fails if there isn't one)
	@if [ -z "$(DEPLOY_HOST)" ]; then \
		printf "$(RED)>> DEPLOY_HOST is not set.$(RESET)\n"; \
		printf "   Add it to .env or run: make deploy DEPLOY_HOST=root@your.server\n"; \
		exit 1; \
	fi
	@# A dirty tree means the published image is not the code you are looking at.
	@if [ -n "$$(git status --porcelain)" ] && [ -z "$(ALLOW_DIRTY)" ]; then \
		printf "$(RED)>> Working tree is dirty — the built image cannot match it.$(RESET)\n"; \
		git status --short; \
		printf "   Commit and push, or override with: make deploy ALLOW_DIRTY=1\n"; \
		exit 1; \
	fi
	@# CI builds from the remote, so an unpushed commit has no image either.
	@git fetch -q origin main 2>/dev/null || true
	@if ! git merge-base --is-ancestor HEAD origin/main 2>/dev/null; then \
		printf "$(RED)>> HEAD ($(GIT_SHA)) is not on origin/main, so CI never built it.$(RESET)\n"; \
		printf "   Run: git push origin main && make release\n"; \
		exit 1; \
	fi
	@printf "$(BLUE)>> Syncing deploy/ to $(DEPLOY_HOST)...$(RESET)\n"
	rsync -az --exclude '.env' deploy $(DEPLOY_HOST):$(dir $(DEPLOY_DIR))
	@printf "$(BLUE)>> Pulling $(GIT_SHA)...$(RESET)\n"
	@if ! ssh $(DEPLOY_HOST) 'cd $(DEPLOY_DIR) && $(PINNED_IMAGES) $(COMPOSE) pull'; then \
		printf "$(RED)>> No published image tagged $(GIT_SHA).$(RESET)\n"; \
		printf "   docker-push.yml only runs on demand — run: make release\n"; \
		exit 1; \
	fi
	@printf "$(BLUE)>> Recreating containers...$(RESET)\n"
	@# --force-recreate every service: `up -d` alone leaves containers running on
	@# a stale image even after a successful pull, which silently ships old code.
	ssh $(DEPLOY_HOST) 'cd $(DEPLOY_DIR) && $(PINNED_IMAGES) $(COMPOSE) up -d --force-recreate'
	@# Confirm what is actually running, rather than trusting that it worked.
	@running=$$(ssh $(DEPLOY_HOST) 'cd $(DEPLOY_DIR) && $(PINNED_IMAGES) $(COMPOSE) ps --format "{{.Image}}"' \
		| grep -c ":$(GIT_SHA)$$" || true); \
	if [ "$$running" -lt 2 ]; then \
		printf "$(RED)>> server and web are not both running $(GIT_SHA) (matched $$running).$(RESET)\n"; \
		exit 1; \
	fi
	@printf "$(GREEN)>> Deployed $(GIT_SHA) and verified running.$(RESET)\n"

release: ## Build the image for HEAD in CI, wait for it, then deploy
	@printf "$(BLUE)>> Dispatching image build for $(GIT_SHA)...$(RESET)\n"
	@if ! gh workflow run docker-push.yml --ref main; then \
		printf "$(RED)>> Could not dispatch the build.$(RESET)\n"; \
		printf "   gh is authenticated as: %s\n" "$$(gh api user -q .login 2>/dev/null || echo unknown)"; \
		printf "   That account needs admin on the repo — switch with: gh auth switch --user <account>\n"; \
		exit 1; \
	fi
	@# Wait for the run *for this commit*. Taking "the most recent run" can match
	@# an earlier commit's run that has already finished, which then deploys
	@# stale code with every step reporting success.
	@sha=$$(git rev-parse HEAD); id=""; \
	for i in $$(seq 1 30); do \
		id=$$(gh run list --workflow=docker-push.yml --limit 10 \
			--json databaseId,headSha -q "[.[] | select(.headSha==\"$$sha\")][0].databaseId"); \
		[ -n "$$id" ] && [ "$$id" != "null" ] && break; \
		sleep 5; \
	done; \
	if [ -z "$$id" ] || [ "$$id" = "null" ]; then \
		printf "$(RED)>> No workflow run appeared for $(GIT_SHA).$(RESET)\n"; exit 1; \
	fi; \
	printf "$(BLUE)>> Waiting on run %s...$(RESET)\n" "$$id"; \
	gh run watch "$$id" --exit-status || { \
		printf "$(RED)>> Image build failed for $(GIT_SHA).$(RESET)\n"; exit 1; }
	@$(MAKE) deploy

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
