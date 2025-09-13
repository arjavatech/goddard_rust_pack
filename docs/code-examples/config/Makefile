# Rust Lambda API - Makefile
# Provides convenient commands for building, testing, and deploying

.PHONY: help build test deploy clean setup dev destroy lint format check-deps install-deps

# Default target
help: ## Show this help message
	@echo "Rust Lambda API - Available Commands"
	@echo "===================================="
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z_-]+:.*##/ {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@echo ""
	@echo "Environment Variables:"
	@echo "  AWS_PROFILE     AWS profile to use (default: default)"
	@echo "  AWS_REGION      AWS region (default: us-east-1)"
	@echo "  NODE_ENV        Environment (dev/staging/prod)"

# Variables
PROJECT_ROOT := $(shell pwd)
LAMBDA_DIR := $(PROJECT_ROOT)/lambda
INFRA_DIR := $(PROJECT_ROOT)/infrastructure
SCRIPTS_DIR := $(PROJECT_ROOT)/scripts

AWS_PROFILE ?= default
AWS_REGION ?= us-east-1
NODE_ENV ?= development

# Colors for output
BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[1;33m
RED := \033[0;31m
NC := \033[0m # No Color

# Build targets
build: ## Build both Rust Lambda and CDK infrastructure
	@echo "$(BLUE)[BUILD]$(NC) Building Rust Lambda and CDK infrastructure..."
	@$(SCRIPTS_DIR)/build.sh

build-lambda: ## Build only the Rust Lambda function
	@echo "$(BLUE)[BUILD]$(NC) Building Rust Lambda function..."
	@cd $(LAMBDA_DIR) && cargo lambda build --release

build-infra: ## Build only the CDK infrastructure
	@echo "$(BLUE)[BUILD]$(NC) Building CDK infrastructure..."
	@cd $(INFRA_DIR) && npm run build

# Test targets
test: ## Run all tests (Rust and TypeScript)
	@echo "$(BLUE)[TEST]$(NC) Running all tests..."
	@$(SCRIPTS_DIR)/test-local.sh

test-lambda: ## Run only Rust Lambda tests
	@echo "$(BLUE)[TEST]$(NC) Running Rust tests..."
	@cd $(LAMBDA_DIR) && cargo test

test-infra: ## Run only CDK infrastructure tests
	@echo "$(BLUE)[TEST]$(NC) Running CDK tests..."
	@cd $(INFRA_DIR) && npm test

test-coverage: ## Run tests with coverage reporting
	@echo "$(BLUE)[TEST]$(NC) Running tests with coverage..."
	@cd $(LAMBDA_DIR) && cargo tarpaulin --out Html
	@cd $(INFRA_DIR) && npm run test:coverage

# Development targets
dev: ## Start local development environment
	@echo "$(BLUE)[DEV]$(NC) Starting local development..."
	@$(SCRIPTS_DIR)/test-local.sh --server-only

dev-watch: ## Start development with file watching
	@echo "$(BLUE)[DEV]$(NC) Starting development with file watching..."
	@cd $(LAMBDA_DIR) && cargo lambda watch

# Deployment targets
deploy: ## Deploy to AWS
	@echo "$(BLUE)[DEPLOY]$(NC) Deploying to AWS..."
	@AWS_PROFILE=$(AWS_PROFILE) $(SCRIPTS_DIR)/deploy.sh

deploy-auto: ## Deploy to AWS without approval prompts
	@echo "$(BLUE)[DEPLOY]$(NC) Auto-deploying to AWS..."
	@AWS_PROFILE=$(AWS_PROFILE) $(SCRIPTS_DIR)/deploy.sh --auto-approve

destroy: ## Destroy AWS infrastructure
	@echo "$(RED)[DESTROY]$(NC) Destroying AWS infrastructure..."
	@AWS_PROFILE=$(AWS_PROFILE) $(SCRIPTS_DIR)/cleanup.sh --destroy-aws

# Setup targets
setup: install-deps ## Setup the entire project
	@echo "$(GREEN)[SETUP]$(NC) Project setup completed!"

install-deps: ## Install all dependencies
	@echo "$(BLUE)[SETUP]$(NC) Installing dependencies..."
	@echo "$(YELLOW)Installing Rust dependencies...$(NC)"
	@cd $(LAMBDA_DIR) && cargo build
	@echo "$(YELLOW)Installing Node.js dependencies...$(NC)"
	@cd $(INFRA_DIR) && npm install

check-deps: ## Check if required tools are installed
	@echo "$(BLUE)[CHECK]$(NC) Checking dependencies..."
	@command -v cargo >/dev/null 2>&1 || { echo "$(RED)Error: cargo not found$(NC)"; exit 1; }
	@command -v cargo-lambda >/dev/null 2>&1 || { echo "$(RED)Error: cargo-lambda not found$(NC)"; exit 1; }
	@command -v node >/dev/null 2>&1 || { echo "$(RED)Error: node not found$(NC)"; exit 1; }
	@command -v npm >/dev/null 2>&1 || { echo "$(RED)Error: npm not found$(NC)"; exit 1; }
	@command -v cdk >/dev/null 2>&1 || { echo "$(RED)Error: aws-cdk not found$(NC)"; exit 1; }
	@command -v aws >/dev/null 2>&1 || { echo "$(RED)Error: aws cli not found$(NC)"; exit 1; }
	@echo "$(GREEN)All dependencies found!$(NC)"

# Code quality targets
lint: ## Run linting for both Rust and TypeScript
	@echo "$(BLUE)[LINT]$(NC) Running linters..."
	@cd $(LAMBDA_DIR) && cargo clippy -- -D warnings
	@cd $(INFRA_DIR) && npm run lint

format: ## Format code for both Rust and TypeScript
	@echo "$(BLUE)[FORMAT]$(NC) Formatting code..."
	@cd $(LAMBDA_DIR) && cargo fmt
	@cd $(INFRA_DIR) && npm run format

format-check: ## Check code formatting
	@echo "$(BLUE)[FORMAT]$(NC) Checking code formatting..."
	@cd $(LAMBDA_DIR) && cargo fmt -- --check
	@cd $(INFRA_DIR) && npm run format:check

# Clean targets
clean: ## Clean build artifacts
	@echo "$(BLUE)[CLEAN]$(NC) Cleaning build artifacts..."
	@$(SCRIPTS_DIR)/cleanup.sh --local-only

clean-all: ## Clean everything including AWS resources
	@echo "$(RED)[CLEAN]$(NC) Cleaning everything including AWS resources..."
	@$(SCRIPTS_DIR)/cleanup.sh --destroy-aws

# Utility targets
logs: ## Show Lambda function logs
	@echo "$(BLUE)[LOGS]$(NC) Showing Lambda logs..."
	@aws logs tail /aws/lambda/rust-hello-world-api --follow --profile $(AWS_PROFILE)

status: ## Show deployment status
	@echo "$(BLUE)[STATUS]$(NC) Checking deployment status..."
	@cd $(INFRA_DIR) && cdk list
	@aws lambda get-function --function-name rust-hello-world-api --profile $(AWS_PROFILE) --region $(AWS_REGION) || echo "Function not deployed"

endpoints: ## Show API endpoints
	@echo "$(BLUE)[INFO]$(NC) API Endpoints:"
	@cd $(INFRA_DIR) && cdk output --profile $(AWS_PROFILE) || echo "Stack not deployed"

# Load testing
load-test: ## Run load tests against deployed API
	@echo "$(BLUE)[LOAD-TEST]$(NC) Running load tests..."
	@$(SCRIPTS_DIR)/test-local.sh --load-test

benchmark: ## Run performance benchmarks
	@echo "$(BLUE)[BENCHMARK]$(NC) Running benchmarks..."
	@cd $(LAMBDA_DIR) && cargo bench
	@$(SCRIPTS_DIR)/test-local.sh --benchmark

# Documentation
docs: ## Generate documentation
	@echo "$(BLUE)[DOCS]$(NC) Generating documentation..."
	@cd $(LAMBDA_DIR) && cargo doc --no-deps --open

# Security
security-audit: ## Run security audits
	@echo "$(BLUE)[SECURITY]$(NC) Running security audits..."
	@cd $(LAMBDA_DIR) && cargo audit
	@cd $(INFRA_DIR) && npm audit

# CI/CD helpers
ci-build: ## Build for CI environment
	@echo "$(BLUE)[CI-BUILD]$(NC) Building for CI..."
	@$(SCRIPTS_DIR)/build.sh --skip-tests

ci-test: ## Run tests for CI environment
	@echo "$(BLUE)[CI-TEST]$(NC) Running CI tests..."
	@$(SCRIPTS_DIR)/test-local.sh
	@cd $(INFRA_DIR) && npm run test:coverage

ci-deploy: ## Deploy for CI environment
	@echo "$(BLUE)[CI-DEPLOY]$(NC) Deploying for CI..."
	@$(SCRIPTS_DIR)/deploy.sh --auto-approve --skip-test

# Version management
version: ## Show version information
	@echo "$(BLUE)[VERSION]$(NC) Version Information:"
	@echo "Rust: $$(rustc --version)"
	@echo "Cargo Lambda: $$(cargo-lambda --version)"
	@echo "Node.js: $$(node --version)"
	@echo "NPM: $$(npm --version)"
	@echo "CDK: $$(cdk --version)"
	@echo "AWS CLI: $$(aws --version)"

# Quick commands
quick-deploy: build deploy ## Quick build and deploy
	@echo "$(GREEN)[SUCCESS]$(NC) Quick deployment completed!"

quick-test: build test ## Quick build and test
	@echo "$(GREEN)[SUCCESS]$(NC) Quick testing completed!"

# Environment-specific targets
dev-deploy: ## Deploy to development environment
	@NODE_ENV=development $(MAKE) deploy

staging-deploy: ## Deploy to staging environment
	@NODE_ENV=staging $(MAKE) deploy

prod-deploy: ## Deploy to production environment
	@NODE_ENV=production $(MAKE) deploy

# Monitoring
monitor: ## Monitor the application
	@echo "$(BLUE)[MONITOR]$(NC) Monitoring application..."
	@echo "CloudWatch Dashboard: https://console.aws.amazon.com/cloudwatch/home?region=$(AWS_REGION)#dashboards:"
	@echo "Lambda Function: https://console.aws.amazon.com/lambda/home?region=$(AWS_REGION)#/functions/rust-hello-world-api"
	@$(MAKE) logs

# Help targets
help-advanced: ## Show advanced help
	@echo "Advanced Usage Examples:"
	@echo ""
	@echo "  Full development cycle:"
	@echo "    make setup && make dev"
	@echo ""
	@echo "  CI/CD pipeline:"
	@echo "    make ci-build && make ci-test && make ci-deploy"
	@echo ""
	@echo "  Production deployment:"
	@echo "    make clean && make build && make test && make prod-deploy"
	@echo ""
	@echo "  Monitoring and debugging:"
	@echo "    make status && make logs && make monitor"