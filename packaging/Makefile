# OS packaging entrypoint for oxydraw. `make build` produces the amd64 .deb via Docker in
# two steps: (1) `frontend` builds the SPA assets with the official multi-arch Bun image on
# the host's NATIVE arch — Bun's JIT segfaults under amd64 emulation, and the assets are
# arch-independent anyway; (2) the self-contained amd64 build image (docker/Dockerfile.build
# — debian + dpkg-dev + Rust) compiles the server (embedding those assets) and packages it.
# `deb-build`/`deb-path` are the targets invoked *inside* that container (no nested Docker).
SHELL := /bin/bash
.PHONY: build docker-build frontend deb-image deb-build deb-path clean

DEB_DIR    := $(CURDIR)/deb
DOCKER_DIR := $(CURDIR)/docker
REPO_ROOT  := $(abspath $(CURDIR)/..)

BUILD_IMAGE     ?= oxydraw-deb-build
BUN_IMAGE       ?= oven/bun:1
# The build image and the apt pool are amd64-only.
DOCKER_PLATFORM ?= linux/amd64

build: docker-build ## Build the .deb on the host via Docker

docker-build: frontend deb-image
	docker run --rm --platform $(DOCKER_PLATFORM) \
	  -v "$(REPO_ROOT):/src" -w /src/packaging $(BUILD_IMAGE) deb-build
	@echo "Built:"; $(MAKE) --no-print-directory deb-path

# Frontend SPA assets (arch-independent). Built with the official multi-arch Bun image on
# the host's native arch — no --platform override, so no emulation. vite writes the assets
# into backend/crates/server/assets, where rust-embed picks them up at compile time.
frontend:
	docker run --rm \
	  -v "$(REPO_ROOT):/src" -w /src/frontend $(BUN_IMAGE) \
	  sh -c "bun install --frozen-lockfile && bun run build"

deb-image:
	docker build --platform $(DOCKER_PLATFORM) \
	  -t $(BUILD_IMAGE) -f $(DOCKER_DIR)/Dockerfile.build $(DOCKER_DIR)

# Invoked inside the build image (no nested Docker).
deb-build:
	@$(MAKE) -C $(DEB_DIR) build

deb-path:
	@$(MAKE) --no-print-directory -C $(DEB_DIR) deb-path

clean:
	@$(MAKE) -C $(DEB_DIR) clean
