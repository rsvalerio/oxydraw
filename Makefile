# Developer entrypoint for oxydraw. `make help` lists targets.
# OS packaging (.deb) lives in packaging/Makefile (`make deb`).
SHELL := /bin/bash
.PHONY: help build release run test fmt clippy check deny cog-check dist-plan pre-release frontend docker docker-run deb clean

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

build: ## Compile (debug)
	cd backend && cargo build --all --all-features

release: frontend ## Compile (release)
	cd backend && cargo build --release --bin oxydraw

run: ## Run the server (http://localhost:3002)
	cd backend && PUBLIC_URL='http://localhost:3002' cargo run
test: ## Run the test suite
	cd backend && cargo test --all --all-features

fmt: ## Format all crates
	cd backend && cargo fmt --all

clippy: ## Lint, warnings as errors
	cd backend && cargo clippy --all --all-features -- -D warnings

check: ## All gates: fmt + clippy + test
	cd backend && ops verify qa;
	cd frontend && ops verify qa;

deny: ## Dependency audit: licenses, advisories, bans (deny.toml)
	cd backend && cargo deny check

cog-check: ## Validate conventional commits since last tag + dry-run the version bump
	@command -v cog >/dev/null || { echo "cog not found — install with: brew install cocogitto"; exit 1; }
	@if git describe --tags --abbrev=0 >/dev/null 2>&1; then cog check --from-latest-tag; \
	else echo "no tags yet — skipping cog check (bump dry-run still parses commits)"; fi
	cog bump --auto --dry-run

dist-plan: ## Validate dist-workspace.toml and show release artifacts
	@command -v dist >/dev/null || { echo "dist not found — install with: brew install cargo-dist"; exit 1; }
	dist plan

pre-release: check deny cog-check dist-plan ## Everything CI + the release pipeline will run, locally

# Builds the frontend/ SPA (our custom frontend on @excalidraw/excalidraw) into
# backend/crates/server/assets/, where rust-embed picks it up at `cargo build` time. Always built
# inside the oven/bun Docker image — so the host needs only docker, no Node/bun toolchain.
# `make docker` builds the Dockerfile's `frontend` stage the same way (bun in a container).
ASSETS := backend/crates/server/assets
BUN_IMAGE := oven/bun:1
frontend: ## Build the frontend/ SPA into backend/crates/server/assets/ (Docker)
	@echo "==> building frontend/ (output: $(ASSETS))"
	@command -v docker >/dev/null 2>&1 || { echo "docker is required to build the frontend" >&2; exit 1; }
	@echo "==> building in $(BUN_IMAGE)"
	docker run --rm -u $$(id -u):$$(id -g) -e HOME=/tmp \
		-v "$(CURDIR)":/app -w /app/frontend \
		$(BUN_IMAGE) sh -c "bun install --frozen-lockfile && bun run build"
	@echo "==> done: $$(du -sh $(ASSETS) | cut -f1) embedded. Now run: make release"

docker: ## Build the Docker image
	docker build -t oxydraw .

docker-run: docker ## Run the Docker image on port 3002
	docker run --rm -p 3002:3002 oxydraw

deb: ## Build the .deb package (see packaging/)
	@test -d packaging || { echo "packaging/ not found — it is referenced by publish-deb.yml and backend/crates/server/Cargo.toml but missing from the repo"; exit 1; }
	$(MAKE) -C packaging build

clean: ## Remove build artifacts
	cd backend && cargo clean
	@if [ -d packaging ]; then $(MAKE) -C packaging clean; fi
