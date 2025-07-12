# Project metadata
CRATE_NAME := squishrs
CARGO := cargo
TARGET := target/release/$(CRATE_NAME)

# Default target
.PHONY: all
all: build ## Build the release binary

.PHONY: build
build: # Build binary
	$(CARGO) build --release

.PHONY: run
run: # Run target file
	$(TARGET) $(ARGS)

.PHONY: test
test: ## Run tests
	$(CARGO) test

.PHONY: fmt
fmt: ## Format code with rustfmt
	$(CARGO) fmt

.PHONY: lint
lint: ## Run clippy linter
	$(CARGO) clippy --all-targets --all-features -- -D warnings

.PHONY: check
check: ## Run basic type-checking
	$(CARGO) check

.PHONY: clean
clean: ## Clean build artifacts
	$(CARGO) clean

.PHONY: install
install: ## Install the binary system-wide
	$(CARGO) install --path .

.PHONY: help
help: ## Show help for each target
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'
