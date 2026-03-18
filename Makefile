.PHONY: help test server web up build build-testflight screenshots

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

test: ## Run Playwright tests
	npx playwright test

server: ## Start backend dev server (port 3000)
	cd server && cargo run

web: ## Start frontend dev server (port 5173)
	cd web && npm run dev

build: ## Build server and web for production
	cd server && cargo build --release
	cd web && npm run build

up: ## Start everything with Docker Compose
	docker compose up --build

build-testflight: ## Archive and upload iOS app to TestFlight
	cd ios && ./build-testflight.sh

SCREENSHOT_SIM ?= iPhone 15 Pro Max

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
