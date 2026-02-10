#!/bin/bash
# Validation functions for preflight checks
# Provides modular, reusable validation logic

# Source common utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

# =============================================
# FLY.IO VALIDATION
# =============================================

validate_flyio_auth() {
    print_step "Checking Fly.io authentication"

    if ! check_command "fly" "brew install flyctl"; then
        return 1
    fi

    if ! fly auth whoami &>/dev/null; then
        print_error "Not authenticated with Fly.io"
        print_info "Run: fly auth login"
        return 1
    fi

    local whoami
    whoami=$(fly auth whoami 2>/dev/null | head -n 1)
    print_success "Authenticated as: $whoami"
    return 0
}

validate_flyio_app() {
    local app_name="$1"

    print_step "Checking Fly.io app: $app_name"

    if ! fly apps list 2>/dev/null | grep -q "$app_name"; then
        print_error "Fly.io app not found: $app_name"
        print_info "Available apps:"
        fly apps list 2>/dev/null | tail -n +2
        return 1
    fi

    print_success "App found: $app_name"
    return 0
}

# =============================================
# DATABASE VALIDATION
# =============================================

validate_database_url() {
    print_step "Validating DATABASE_URL format"

    if [[ -z "$DATABASE_URL" ]]; then
        print_error "DATABASE_URL not set"
        return 1
    fi

    # Check for postgresql:// prefix
    if [[ ! "$DATABASE_URL" =~ ^postgresql:// ]]; then
        print_error "DATABASE_URL must start with postgresql://"
        return 1
    fi

    # Check for basic components (user@host:port/database)
    if [[ ! "$DATABASE_URL" =~ postgresql://[^@]+@[^:]+:[0-9]+/.+ ]]; then
        print_error "DATABASE_URL format invalid (expected: postgresql://user:pass@host:port/db)"
        return 1
    fi

    print_success "DATABASE_URL format valid"
    return 0
}

check_db_connection() {
    print_step "Testing database connection"

    if ! check_command "psql" "brew install postgresql"; then
        print_warning "psql not available, skipping connection test"
        return 0
    fi

    if ! validate_database_url; then
        return 1
    fi

    if psql "$DATABASE_URL" -c "SELECT 1" &>/dev/null; then
        print_success "Database connection successful"
        return 0
    else
        print_error "Database connection failed"
        print_info "Check DATABASE_URL and network connectivity"
        return 1
    fi
}

check_pending_migrations() {
    print_step "Checking for pending migrations"

    local migrations_dir="database/migrations"
    local migration_files

    if [[ -d "$migrations_dir" ]]; then
        migration_files=$(find "$migrations_dir" -name "*.sql" 2>/dev/null | wc -l)
        if [[ $migration_files -gt 0 ]]; then
            print_warning "Found $migration_files migration file(s) in $migrations_dir"
            print_info "These will be applied during deployment"
        else
            print_success "No migration files found"
        fi
    else
        print_info "No migrations directory found (not required)"
    fi

    return 0
}

# =============================================
# RUST TOOLCHAIN VALIDATION
# =============================================

check_rust_installed() {
    print_step "Checking Rust toolchain"

    if ! check_command "rustc" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; then
        return 1
    fi

    if ! check_command "cargo" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; then
        return 1
    fi

    local rust_version
    rust_version=$(rustc --version 2>/dev/null)
    print_success "Rust installed: $rust_version"
    return 0
}

check_cargo_lambda() {
    print_step "Checking cargo-lambda"

    if ! check_command "cargo-lambda" "brew tap cargo-lambda/cargo-lambda && brew install cargo-lambda"; then
        print_warning "cargo-lambda not installed (optional for local builds)"
        return 0  # Not critical
    fi

    local version
    version=$(cargo-lambda --version 2>/dev/null)
    print_success "cargo-lambda installed: $version"
    return 0
}

# =============================================
# DOCKER VALIDATION
# =============================================

check_docker_running() {
    print_step "Checking Docker"

    if ! check_command "docker" "brew install docker"; then
        return 1
    fi

    if ! docker info &>/dev/null; then
        print_error "Docker daemon not running"
        print_info "Start Docker Desktop or run: sudo systemctl start docker"
        return 1
    fi

    local docker_version
    docker_version=$(docker --version 2>/dev/null)
    print_success "Docker running: $docker_version"
    return 0
}

# =============================================
# ENVIRONMENT FILE VALIDATION
# =============================================

check_env_file() {
    local env_file="${1:-.env}"

    print_step "Checking environment file: $env_file"

    if [[ ! -f "$env_file" ]]; then
        print_error "Environment file not found: $env_file"
        print_info "Create from template: cp .env.example $env_file"
        return 1
    fi

    print_success "Environment file exists"
    return 0
}

check_required_env_vars() {
    local env_vars=(
        "DATABASE_URL"
        "SUPABASE_URL"
        "SUPABASE_ANON_KEY"
        "SUPABASE_SERVICE_ROLE_KEY"
        "JWT_SECRET"
        "OWNER_API_KEY"
    )

    print_step "Validating required environment variables"

    local missing=0
    for var in "${env_vars[@]}"; do
        if [[ -z "${!var}" ]]; then
            print_error "Missing required variable: $var"
            missing=$((missing + 1))
        fi
    done

    if [[ $missing -gt 0 ]]; then
        print_error "Missing $missing required environment variable(s)"
        return 1
    fi

    print_success "All required environment variables set"
    return 0
}

# =============================================
# GIT VALIDATION
# =============================================

check_git_clean() {
    print_step "Checking git working directory"

    if ! command -v git &>/dev/null; then
        print_warning "Git not installed, skipping check"
        return 0
    fi

    if ! git rev-parse --git-dir &>/dev/null; then
        print_warning "Not a git repository, skipping check"
        return 0
    fi

    local status
    status=$(git status --porcelain 2>/dev/null)

    if [[ -n "$status" ]]; then
        print_warning "Git working directory has uncommitted changes"
        print_info "Consider committing changes before deployment"
        # Not an error, just a warning
    else
        print_success "Git working directory clean"
    fi

    local branch
    branch=$(get_git_branch)
    print_info "Current branch: $branch"

    return 0
}

# =============================================
# BUILD VALIDATION
# =============================================

check_rust_project_compiles() {
    print_step "Validating Rust project compiles"

    local project_dir="lambda/goddard"

    if [[ ! -d "$project_dir" ]]; then
        print_error "Project directory not found: $project_dir"
        return 1
    fi

    print_info "Running cargo check (this may take a moment)..."

    if (cd "$project_dir" && cargo check --quiet 2>&1 | tail -n 5); then
        print_success "Rust project compiles successfully"
        return 0
    else
        print_error "Rust project has compilation errors"
        print_info "Run 'cd $project_dir && cargo check' for details"
        return 1
    fi
}

check_dockerfile_exists() {
    local dockerfile="${1:-lambda/goddard/Dockerfile}"

    print_step "Checking Dockerfile exists"

    if [[ ! -f "$dockerfile" ]]; then
        print_error "Dockerfile not found: $dockerfile"
        return 1
    fi

    print_success "Dockerfile found: $dockerfile"
    return 0
}

# =============================================
# FLY.IO CONFIG VALIDATION
# =============================================

check_flyio_config() {
    local fly_config="$1"

    print_step "Validating Fly.io config: $fly_config"

    if [[ ! -f "$fly_config" ]]; then
        print_error "Fly.io config not found: $fly_config"
        return 1
    fi

    # Basic validation - check for app name
    if ! grep -q "app = " "$fly_config"; then
        print_error "Invalid fly.toml - missing app name"
        return 1
    fi

    local app_name
    app_name=$(grep "app = " "$fly_config" | head -n 1 | sed 's/app = "\(.*\)"/\1/')
    print_success "Fly.io config valid for app: $app_name"

    return 0
}

# =============================================
# COMPREHENSIVE PREFLIGHT CHECK
# =============================================

run_all_preflight_checks() {
    local env="${1:-dev}"
    local env_file=".env"
    local fly_config="lambda/goddard/fly.toml"
    local fly_app="goddard"

    # Determine environment-specific settings
    if [[ "$env" == "production" ]]; then
        env_file=".env.production"
        fly_config="lambda/goddard/fly.toml.production"
        fly_app="goddard-falling-surf-1798"
    elif [[ "$env" == "dev" ]]; then
        env_file=".env.dev"
    fi

    print_header "🔍 Running Preflight Checks for $env environment"

    local failed=0

    # Load environment first
    if ! check_env_file "$env_file"; then
        failed=$((failed + 1))
    else
        load_environment "$env_file" || failed=$((failed + 1))
    fi

    # Run all checks
    check_required_env_vars || failed=$((failed + 1))
    validate_database_url || failed=$((failed + 1))
    validate_flyio_auth || failed=$((failed + 1))
    validate_flyio_app "$fly_app" || failed=$((failed + 1))
    check_rust_installed || failed=$((failed + 1))
    check_docker_running || failed=$((failed + 1))
    check_cargo_lambda  # Optional, don't count failure
    check_flyio_config "$fly_config" || failed=$((failed + 1))
    check_dockerfile_exists || failed=$((failed + 1))
    check_db_connection || failed=$((failed + 1))
    check_pending_migrations  # Informational only
    check_git_clean  # Warning only
    check_rust_project_compiles || failed=$((failed + 1))

    echo ""
    if [[ $failed -eq 0 ]]; then
        print_success "All preflight checks passed! ✨"
        return 0
    else
        print_error "Preflight checks failed: $failed error(s)"
        print_info "Fix the errors above and try again"
        return 1
    fi
}

# Export all validation functions
export -f validate_flyio_auth
export -f validate_flyio_app
export -f validate_database_url
export -f check_db_connection
export -f check_pending_migrations
export -f check_rust_installed
export -f check_cargo_lambda
export -f check_docker_running
export -f check_env_file
export -f check_required_env_vars
export -f check_git_clean
export -f check_rust_project_compiles
export -f check_dockerfile_exists
export -f check_flyio_config
export -f run_all_preflight_checks
