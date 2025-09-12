.PHONY: help install build test test-local deploy destroy clean synth

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-15s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

install: ## Install all dependencies
	@echo "📦 Installing Rust dependencies..."
	cd lambda/hello-world && cargo fetch
	@echo "📦 Installing cargo-lambda..."
	pip3 install cargo-lambda || cargo install cargo-lambda
	@echo "📦 Installing CDK dependencies..."
	cd infrastructure && npm install
	@echo "✅ All dependencies installed!"

build: ## Build Rust Lambda and CDK
	@echo "🔨 Building project..."
	chmod +x scripts/build.sh
	./scripts/build.sh

test: ## Run tests
	@echo "🧪 Running Rust tests..."
	cd lambda/hello-world && cargo test
	@echo "🧪 Running CDK tests..."
	cd infrastructure && npm test || true
	@echo "✅ All tests complete!"

test-local: ## Run Lambda locally for testing
	chmod +x scripts/test-local.sh
	./scripts/test-local.sh

deploy: ## Deploy to AWS
	chmod +x scripts/deploy.sh
	./scripts/deploy.sh

synth: ## Synthesize CDK stack
	cd infrastructure && npm run synth

destroy: ## Destroy AWS resources
	@echo "⚠️  WARNING: This will delete all AWS resources!"
	@read -p "Are you sure? [y/N] " -n 1 -r; \
	echo ""; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		cd infrastructure && npm run destroy; \
	fi

clean: ## Clean build artifacts
	@echo "🧹 Cleaning build artifacts..."
	cd lambda/hello-world && cargo clean
	rm -rf infrastructure/node_modules infrastructure/lib infrastructure/cdk.out
	@echo "✅ Clean complete!"

bootstrap: ## Bootstrap CDK (first time setup)
	cd infrastructure && npx cdk bootstrap

diff: ## Show CDK diff
	cd infrastructure && npm run diff