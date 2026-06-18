# oxydraw .deb — compiles the server binary (embedding the prebuilt frontend assets),
# then stages and packages with dpkg-deb. Runs INSIDE the build image (see
# ../docker/Dockerfile.build), which provides dpkg-dev + the Rust toolchain. The repo is
# mounted at /src. The frontend assets are built separately (parent Makefile `frontend`
# target) because Bun's JIT segfaults under amd64 emulation.
SHELL := /bin/bash
.PHONY: build clean deb-path

DEB_DIR   := $(CURDIR)
REPO_ROOT := $(abspath $(DEB_DIR)/../..)
PACKAGING := $(abspath $(DEB_DIR)/..)
# Stage on the container-local filesystem, NOT under the /src bind mount: chmod/ownership
# on the macOS virtiofs mount does not reliably propagate, so dpkg-deb intermittently sees
# maintainer scripts as 0644. Only the finished .deb is written back to OUTPUT (on /src).
STAGING   := /tmp/oxydraw-pkg/staging
OUTPUT    := $(DEB_DIR)/.build

PKG_NAME := oxydraw
# Single source of truth: the workspace version in backend/Cargo.toml (kept in sync by cocogitto).
VERSION  := $(shell grep -m1 '^version' $(REPO_ROOT)/backend/Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')
ARCH     := amd64
DEB      := $(OUTPUT)/$(PKG_NAME)_$(VERSION)_$(ARCH).deb

BIN        := $(REPO_ROOT)/backend/target/release/oxydraw
ASSETS_DIR := $(REPO_ROOT)/backend/crates/server/assets

build: $(DEB)

deb-path:
	@echo "$(DEB)"

$(DEB): $(DEB_DIR)/control.in $(DEB_DIR)/conffiles \
		$(DEB_DIR)/postinst $(DEB_DIR)/prerm $(DEB_DIR)/postrm \
		$(PACKAGING)/oxydraw.env $(PACKAGING)/systemd/oxydraw.service
	@if [ -z "$(VERSION)" ]; then echo "Could not determine VERSION from $(REPO_ROOT)/backend/Cargo.toml" >&2; exit 1; fi
	# --- 1. Require the embedded frontend assets (rust-embed reads $(ASSETS_DIR) at
	#         compile time). They are built separately by the `frontend` target in the
	#         parent Makefile, which runs Bun on the host's native arch (Bun's JIT
	#         segfaults under amd64 emulation, so it cannot run in this image). ---
	@if [ ! -f "$(ASSETS_DIR)/index.html" ]; then \
		echo "Frontend assets missing in $(ASSETS_DIR) — run 'make -C $(PACKAGING) frontend' first" >&2; \
		exit 1; \
	fi
	# --- 2. Compile the server binary. ---
	cd "$(REPO_ROOT)/backend" && cargo build --release --bin oxydraw
	# --- 3. Stage the package tree. ---
	@rm -rf "$(STAGING)"
	@mkdir -p "$(OUTPUT)" \
		"$(STAGING)/DEBIAN" \
		"$(STAGING)/usr/bin" \
		"$(STAGING)/etc/oxydraw" \
		"$(STAGING)/etc/systemd/system" \
		"$(STAGING)/usr/share/doc/oxydraw"
	sed 's/@VERSION@/$(VERSION)/' "$(DEB_DIR)/control.in" > "$(STAGING)/DEBIAN/control"
	cp "$(DEB_DIR)/conffiles" "$(STAGING)/DEBIAN/conffiles"
	cp "$(DEB_DIR)/postinst" "$(DEB_DIR)/prerm" "$(DEB_DIR)/postrm" "$(STAGING)/DEBIAN/"
	chmod 755 "$(STAGING)/DEBIAN/postinst" "$(STAGING)/DEBIAN/prerm" "$(STAGING)/DEBIAN/postrm"
	install -m 0755 "$(BIN)" "$(STAGING)/usr/bin/oxydraw"
	install -m 0640 "$(PACKAGING)/oxydraw.env" "$(STAGING)/etc/oxydraw/oxydraw.env"
	install -m 0644 "$(PACKAGING)/systemd/oxydraw.service" "$(STAGING)/etc/systemd/system/oxydraw.service"
	install -m 0644 "$(REPO_ROOT)/README.md" "$(STAGING)/usr/share/doc/oxydraw/README.md"
	install -m 0644 "$(REPO_ROOT)/docs/DEPLOYMENT.md" "$(STAGING)/usr/share/doc/oxydraw/DEPLOYMENT.md"
	# --- 4. Build the .deb. ---
	dpkg-deb --root-owner-group --build "$(STAGING)" "$(DEB)"
	@echo "Built $(DEB)"

clean:
	rm -rf "$(OUTPUT)" "$(STAGING)"
