VERSION ?= $(shell cargo metadata --no-deps --format-version 1 2>/dev/null | jq -r '.packages[0].version' || sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
TARGET ?= $(shell rustc -Vv | sed -n 's/^host: //p')
DIST_DIR ?= dist
PKG_NAME = agymux-v$(VERSION)-$(TARGET)
PKG_DIR = $(DIST_DIR)/$(PKG_NAME)

.PHONY: all build check test lint fmt fmt-check bench clean dist package checksums install uninstall cross-check cross-build help

all: build

help:
	@echo "agymux makefile targets:"
	@echo "  build         - Build release binaries for host target"
	@echo "  check         - Run cargo check across all targets and features"
	@echo "  test          - Run test suite across all targets and features"
	@echo "  lint          - Run cargo clippy with -D warnings"
	@echo "  fmt           - Format code with cargo fmt"
	@echo "  fmt-check     - Check code formatting"
	@echo "  bench         - Compile benchmarks without running"
	@echo "  dist          - Build release and create tar.gz package + sha256 checksum"
	@echo "  checksums     - Regenerate SHA256SUMS for all archives in dist/"
	@echo "  cross-check   - Run cross check for musl and aarch64 targets"
	@echo "  cross-build   - Build musl and aarch64 release binaries using cross"
	@echo "  install       - Install agymux and ax into ~/.local/bin"
	@echo "  uninstall     - Remove agymux and ax from ~/.local/bin"
	@echo "  clean         - Clean build target directory and dist/"

build:
	@cargo build --release

check:
	@cargo check --all-targets --all-features

test:
	@cargo test --all-targets --all-features

lint:
	@cargo clippy --all-targets --all-features -- -D warnings

fmt:
	@cargo fmt --all

fmt-check:
	@cargo fmt --all -- --check

bench:
	@cargo bench --no-run

clean:
	@cargo clean
	@rm -rf $(DIST_DIR)

package: dist

dist: build
	@mkdir -p $(PKG_DIR)/config/tmux
	@cp target/release/agymux $(PKG_DIR)/
	@cp target/release/ax $(PKG_DIR)/
	@cp README.md LICENSE $(PKG_DIR)/
	@cp config/tmux/agymux.conf $(PKG_DIR)/config/tmux/
	@cp -P config/tmux/agymux-tabs.conf $(PKG_DIR)/config/tmux/
	@cp install.sh $(PKG_DIR)/
	@chmod +x $(PKG_DIR)/agymux $(PKG_DIR)/ax $(PKG_DIR)/install.sh
	@tar -czf $(DIST_DIR)/$(PKG_NAME).tar.gz -C $(DIST_DIR) $(PKG_NAME)
	@rm -rf $(PKG_DIR)
	@cd $(DIST_DIR) && (sha256sum $(PKG_NAME).tar.gz 2>/dev/null || shasum -a 256 $(PKG_NAME).tar.gz) > $(PKG_NAME).tar.gz.sha256
	@$(MAKE) checksums
	@echo "Created package: $(DIST_DIR)/$(PKG_NAME).tar.gz"

checksums:
	@if [ -d "$(DIST_DIR)" ]; then \
		cd $(DIST_DIR) && \
		(sha256sum *.tar.gz 2>/dev/null || shasum -a 256 *.tar.gz) > SHA256SUMS 2>/dev/null || true; \
		echo "Generated $(DIST_DIR)/SHA256SUMS"; \
	fi

cross-check:
	@which cross >/dev/null 2>&1 || (echo "Error: 'cross' command not found. Install via: cargo install cross" && exit 1)
	@echo "==> Running cross check for x86_64-unknown-linux-musl..."
	@cross check --target x86_64-unknown-linux-musl
	@echo "==> Running cross check for aarch64-unknown-linux-gnu..."
	@cross check --target aarch64-unknown-linux-gnu

cross-build:
	@which cross >/dev/null 2>&1 || (echo "Error: 'cross' command not found. Install via: cargo install cross" && exit 1)
	@echo "==> Building x86_64-unknown-linux-musl..."
	@cross build --release --target x86_64-unknown-linux-musl
	@echo "==> Building aarch64-unknown-linux-gnu..."
	@cross build --release --target aarch64-unknown-linux-gnu

install: build
	@mkdir -p $(HOME)/.local/bin
	@cp target/release/agymux $(HOME)/.local/bin/
	@cp target/release/ax $(HOME)/.local/bin/
	@echo "Installed agymux and ax to $(HOME)/.local/bin"

uninstall:
	@rm -f $(HOME)/.local/bin/agymux $(HOME)/.local/bin/ax
	@echo "Removed agymux and ax from $(HOME)/.local/bin"
