SHELL := bash
CARGO ?= cargo
MANIFEST := MusicPlayer/Cargo.toml
PYTHON ?= python3

.DEFAULT_GOAL := help
.PHONY: help build run test test-rust test-py venv check verify lint fmt fmt-check \
	install install-minimal clean

help: ## List available targets
	@printf 'Targets:\n'
	@grep -hE '^[a-zA-Z0-9_-]+:.*## ' $(MAKEFILE_LIST) | sed -E 's/:.*## /  - /'

build: ## Build the full TUI
	$(CARGO) build --manifest-path $(MANIFEST)

run: ## Run the full TUI from source
	$(CARGO) run --manifest-path $(MANIFEST)

test: test-rust test-py ## Run all Rust and Python tests

test-rust: ## Rust tests (default and minimal feature sets)
	$(CARGO) test --manifest-path $(MANIFEST)
	$(CARGO) test --manifest-path $(MANIFEST) --no-default-features

test-py: venv ## Python test suite
	.venv/bin/python -m pytest -q

venv: .venv/bin/pytest ## Create .venv with runtime and dev dependencies

.venv/bin/pytest: requirements.txt requirements-dev.txt
	$(PYTHON) -m venv .venv
	.venv/bin/python -m pip install -q --upgrade pip
	.venv/bin/python -m pip install -q -r requirements.txt -r requirements-dev.txt
	@touch $@

check: ## Fail on any rustc warning in any feature combination (same as CI)
	bash scripts/check.sh

verify: check test ## Full pre-PR verification (what CI gates)

lint: fmt-check ## rustfmt check and clippy -D warnings
	$(CARGO) clippy --manifest-path $(MANIFEST) --all-targets -- -D warnings

fmt-check:
	$(CARGO) fmt --manifest-path $(MANIFEST) -- --check

fmt: ## Apply rustfmt
	$(CARGO) fmt --manifest-path $(MANIFEST)

install: ## Install the full TUI with cargo
	$(CARGO) install --path MusicPlayer --force

install-minimal: ## Install the lean build (no Python, no network)
	$(CARGO) install --path MusicPlayer --no-default-features --force

clean: ## Remove Rust build artifacts
	$(CARGO) clean --manifest-path $(MANIFEST)
