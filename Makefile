.PHONY: all install uninstall test

all:
	@echo "agymux: Generic Antigravity Terminal Multiplexer"
	@echo "Run 'make install' to link binaries and configure tmux."
	@echo "Run 'make uninstall' to cleanly revert all system state."

install:
	@./install.sh

uninstall:
	@./uninstall.sh

test:
	@bash -n bin/agymux
	@bash -n lib/core/*.sh
	@bash -n lib/ui/*.sh
	@bash -n lib/daemon/*.sh
	@bash -n install.sh
	@bash -n uninstall.sh
	@echo "Syntax verification passed."
