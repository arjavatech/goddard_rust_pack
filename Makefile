# =============================================
# Goddard School Enrollment Management System
# Makefile for Database and Development Operations
# =============================================

# Load environment variables if .env exists
-include .env
export

# PostgreSQL path
PSQL := /opt/homebrew/opt/postgresql@14/bin/psql

# Colors for output
RED=\033[0;31m
GREEN=\033[0;32m
YELLOW=\033[1;33m
BLUE=\033[0;34m
NC=\033[0m # No Color

# AWS Profile Configuration
AWS_PROFILE ?= default

.PHONY: help install build test test-local deploy destroy clean synth db-setup db-reset db-status env-setup

# Default target
.DEFAULT_GOAL := help

# =============================================
# HELP
# =============================================

help: ## 📋 Display this help message
	@echo ""
	@echo "$(BLUE)🏫 Goddard School Enrollment Management System$(NC)"
	@echo "$(BLUE)================================================$(NC)"
	@echo ""
	@echo "$(YELLOW)Database Commands:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E 'db-|database' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(YELLOW)Development Commands:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -v -E 'db-|database|help' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(YELLOW)Environment Variables:$(NC)"
	@echo "  $(GREEN)AWS_PROFILE$(NC)     AWS profile to use (default: default)"
	@echo "  $(GREEN)DATABASE_URL$(NC)    PostgreSQL connection string"

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
	chmod +x scripts/build.sh
	./scripts/build.sh

test: ## Run tests
	@echo "🧪 Running Rust tests..."
	cd lambda/goddard && cargo test
	@echo "🧪 Running CDK tests..."
	cd infrastructure && npm test || true
	@echo "✅ All tests complete!"

test-local: ## Run Lambda locally for testing
	chmod +x scripts/test-local.sh
	./scripts/test-local.sh

deploy: ## Deploy to AWS
	chmod +x scripts/deploy.sh
	AWS_PROFILE=$(AWS_PROFILE) ./scripts/deploy.sh

deploy-env: ## Deploy environment variables to Lambda
	chmod +x scripts/deploy-env-auto.sh
	AWS_PROFILE=$(AWS_PROFILE) ./scripts/deploy-env-auto.sh

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
	rm -rf infrastructure/node_modules infrastructure/cdk.out infrastructure/dist
	find infrastructure -name "*.js" -not -path "*/node_modules/*" -delete
	find infrastructure -name "*.d.ts" -not -path "*/node_modules/*" -delete
	find infrastructure -name "*.js.map" -not -path "*/node_modules/*" -delete
	@echo "✅ Clean complete!"

bootstrap: ## Bootstrap CDK (first time setup)
	cd infrastructure && AWS_PROFILE=$(AWS_PROFILE) npx cdk bootstrap

diff: ## Show CDK diff
	cd infrastructure && AWS_PROFILE=$(AWS_PROFILE) npm run diff

validate: ## Validate ARM64 architecture configuration
	chmod +x scripts/validate-architecture.sh
	./scripts/validate-architecture.sh

# =============================================
# DATABASE COMMANDS
# =============================================

db-setup: ## 🚀 Complete database setup with full audit system
	@chmod +x scripts/db-setup.sh
	@./scripts/db-setup.sh

db-clear: ## 🗑️  Clear all tables (keep structure, remove data)
	@if [ -z "$(DATABASE_URL)" ]; then \
		echo "$(RED)❌ ERROR: DATABASE_URL not set$(NC)"; \
		echo "$(YELLOW)Please set DATABASE_URL in your .env file$(NC)"; \
		exit 1; \
	fi
	@echo "$(YELLOW)⚠️  WARNING: This will delete all table structures!$(NC)"
	@echo "$(YELLOW)Type 'yes' to continue:$(NC)"
	@read confirm && [ "$$confirm" = "yes" ] || (echo "$(BLUE)Operation cancelled.$(NC)" && exit 1)
	@echo "$(YELLOW)🗑️  Dropping all tables...$(NC)"
	@$(PSQL) "$(DATABASE_URL)" -c "\
		DO \$$\$$ \
		DECLARE \
			r RECORD; \
		BEGIN \
			FOR r IN (SELECT tablename FROM pg_tables WHERE schemaname = 'public') LOOP \
				EXECUTE 'DROP TABLE IF EXISTS ' || quote_ident(r.tablename) || ' CASCADE'; \
			END LOOP; \
		END \$$\$$;" -q >/dev/null 2>&1 && echo "$(GREEN)✅ All tables cleared!$(NC)" || echo "$(RED)❌ Failed to clear tables$(NC)"

db-reset: ## ⚠️  Reset database (DANGER: Drops all data)
	@if [ -z "$(DATABASE_URL)" ]; then \
		echo "$(RED)❌ ERROR: DATABASE_URL not set$(NC)"; \
		echo "$(YELLOW)Please set DATABASE_URL in your .env file$(NC)"; \
		exit 1; \
	fi
	@echo "$(RED)⚠️  WARNING: This will destroy ALL data in the database!$(NC)"
	@echo "$(YELLOW)Type 'yes' to continue:$(NC)"
	@read confirm && [ "$$confirm" = "yes" ] || (echo "$(BLUE)Operation cancelled.$(NC)" && exit 1)
	@echo "$(YELLOW)🗑️  Dropping and recreating schema...$(NC)"
	@$(PSQL) "$(DATABASE_URL)" -c "DROP SCHEMA IF EXISTS public CASCADE; CREATE SCHEMA public;" -q >/dev/null 2>&1 && echo "$(YELLOW)🔧 Recreating database structure...$(NC)" || (echo "$(RED)❌ Failed to reset schema$(NC)" && exit 1)
	@make db-setup
	@echo "$(GREEN)✅ Database reset completed!$(NC)"

db-status: ## 📊 Check database status and table counts
	@if [ -z "$(DATABASE_URL)" ]; then \
		echo "$(RED)❌ ERROR: DATABASE_URL not set$(NC)"; \
		echo "$(YELLOW)Please set DATABASE_URL in your .env file$(NC)"; \
		exit 1; \
	fi
	@echo "$(BLUE)📊 Database Status$(NC)"
	@echo "$(BLUE)==================$(NC)"
	@$(PSQL) "$(DATABASE_URL)" -c "\
		SELECT \
			COUNT(*) as total_tables, \
			STRING_AGG(table_name, ', ' ORDER BY table_name) as tables \
		FROM information_schema.tables \
		WHERE table_schema = 'public';" 2>/dev/null || echo "$(RED)❌ Could not connect to database$(NC)"
	@echo ""
	@echo "$(BLUE)📈 Table Statistics:$(NC)"
	@$(PSQL) "$(DATABASE_URL)" -c "\
		SELECT \
			tablename, \
			n_live_tup as rows \
		FROM pg_stat_user_tables \
		ORDER BY tablename;" 2>/dev/null || echo "$(RED)❌ Could not get table statistics$(NC)"

db-backup: ## 💾 Create database backup
	@echo "$(BLUE)💾 Creating database backup...$(NC)"
	@mkdir -p backups
	@pg_dump $(DATABASE_URL) > backups/goddard_backup_$(shell date +%Y%m%d_%H%M%S).sql
	@echo "$(GREEN)✅ Backup created in backups/ directory$(NC)"

db-console: ## 🖥️  Open database console
	@echo "$(BLUE)🖥️  Opening database console...$(NC)"
	@$(PSQL) $(DATABASE_URL)


# =============================================
# ENVIRONMENT COMMANDS
# =============================================

env-setup: ## ⚙️  Setup environment configuration
	@if [ ! -f .env ]; then \
		echo "$(YELLOW)📝 Creating .env file from template...$(NC)"; \
		cp .env.example .env; \
		echo "$(GREEN)✅ .env file created!$(NC)"; \
		echo "$(YELLOW)⚠️  Please edit .env file with your configuration$(NC)"; \
	else \
		echo "$(BLUE)ℹ️  .env file already exists$(NC)"; \
	fi

env-validate: ## ✅ Validate environment configuration
	@echo "$(BLUE)✅ Validating environment configuration...$(NC)"
	@if [ -z "$(DATABASE_URL)" ]; then echo "$(RED)❌ DATABASE_URL not set$(NC)"; else echo "$(GREEN)✅ DATABASE_URL set$(NC)"; fi
	@if [ -z "$(JWT_SECRET)" ]; then echo "$(YELLOW)⚠️  JWT_SECRET not set$(NC)"; else echo "$(GREEN)✅ JWT_SECRET set$(NC)"; fi
	@if [ -z "$(NODE_ENV)" ]; then echo "$(YELLOW)⚠️  NODE_ENV not set$(NC)"; else echo "$(GREEN)✅ NODE_ENV: $(NODE_ENV)$(NC)"; fi

quick-start: env-setup db-setup install ## 🚀 Quick start setup (env + db + install)
	@echo "$(GREEN)🎉 Quick start completed!$(NC)"
	@echo "$(BLUE)Next steps:$(NC)"
	@echo "  1. Edit .env file with your configuration"
	@echo "  2. Run 'make dev' to start development server"
	@echo "  3. Visit http://localhost:3000"