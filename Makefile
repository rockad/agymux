.PHONY: all build install uninstall test

all: build

build:
	@cargo build --release

install: build
	@mkdir -p $(HOME)/.local/bin
	@cp target/release/agymux $(HOME)/.local/bin/
	@cp target/release/ax $(HOME)/.local/bin/
	@echo "Installed agymux and ax to $(HOME)/.local/bin"

uninstall:
	@rm -f $(HOME)/.local/bin/agymux $(HOME)/.local/bin/ax
	@echo "Removed agymux and ax from $(HOME)/.local/bin"

test:
	@cargo test
