.PHONY: help test server web up build

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
