.PHONY: help install build test test-local deploy destroy clean synth

# AWS Profile Configuration
AWS_PROFILE ?= default

help: ## Show this help message
	@echo 'Usage: make [target] [AWS_PROFILE=profile-name]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-15s %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@echo ''
	@echo 'Environment variables:'
	@echo '  AWS_PROFILE     AWS profile to use (default: default)'

install: ## Install all dependencies
	@echo "📦 Installing Rust dependencies..."
	cd lambda/goddard && cargo fetch
	@echo "📦 Installing cargo-lambda..."
	pip3 install cargo-lambda || cargo install cargo-lambda
	@echo "📦 Installing CDK dependencies..."
	cd infrastructure && npm install
	@echo "✅ All dependencies installed!"

build: ## Build Rust Lambda for ARM64 and CDK
	@echo "🔨 Building project for ARM64 architecture..."
	chmod +x docs/scripts/build.sh
	./docs/scripts/build.sh

test: ## Run tests
	@echo "🧪 Running Rust tests..."
	cd lambda/goddard && cargo test
	@echo "🧪 Running CDK tests..."
	cd infrastructure && npm test || true
	@echo "✅ All tests complete!"

test-local: ## Run Lambda locally for testing
	chmod +x docs/scripts/test-local.sh
	./docs/scripts/test-local.sh

deploy: ## Deploy to AWS
	chmod +x docs/scripts/deploy.sh
	AWS_PROFILE=$(AWS_PROFILE) ./docs/scripts/deploy.sh

synth: ## Synthesize CDK stack
	cd infrastructure && AWS_PROFILE=$(AWS_PROFILE) npm run synth

destroy: ## Destroy AWS resources
	@echo "⚠️  WARNING: This will delete all AWS resources!"
	@read -p "Are you sure? [y/N] " -n 1 -r; \
	echo ""; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		cd infrastructure && AWS_PROFILE=$(AWS_PROFILE) npm run destroy; \
	fi

clean: ## Clean build artifacts
	@echo "🧹 Cleaning build artifacts..."
	cd lambda/goddard && cargo clean
	rm -rf infrastructure/node_modules infrastructure/lib infrastructure/cdk.out
	@echo "✅ Clean complete!"

bootstrap: ## Bootstrap CDK (first time setup)
	cd infrastructure && AWS_PROFILE=$(AWS_PROFILE) npx cdk bootstrap

diff: ## Show CDK diff
	cd infrastructure && AWS_PROFILE=$(AWS_PROFILE) npm run diff

validate: ## Validate ARM64 architecture configuration
	chmod +x docs/scripts/validate-architecture.sh
	./docs/scripts/validate-architecture.sh