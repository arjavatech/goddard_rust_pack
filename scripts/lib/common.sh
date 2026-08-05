#!/bin/bash
# Shared utility functions for deployment scripts
# Reuses Makefile color scheme and provides common operations

# =============================================
# COLOR DEFINITIONS (Matching Makefile)
# =============================================
export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export NC='\033[0m' # No Color

# =============================================
# LOGGING FUNCTIONS
# =============================================

print_header() {
    local message="$1"
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}${message}${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

print_step() {
    local message="$1"
    echo -e "${BLUE}▶ ${message}${NC}"
}

print_success() {
    local message="$1"
    echo -e "${GREEN}✓ ${message}${NC}"
}

print_error() {
    local message="$1"
    echo -e "${RED}✗ ${message}${NC}" >&2
}

print_warning() {
    local message="$1"
    echo -e "${YELLOW}⚠ ${message}${NC}"
}

print_info() {
    local message="$1"
    echo -e "${BLUE}ℹ ${message}${NC}"
}

# =============================================
# ENVIRONMENT MANAGEMENT
# =============================================

load_environment() {
    local env_file="$1"

    if [[ ! -f "$env_file" ]]; then
        print_error "Environment file not found: $env_file"
        return 1
    fi

    print_step "Loading environment from $env_file"

    # Export all variables from env file
    set -a
    source "$env_file"
    set +a

    print_success "Environment loaded"
    return 0
}

# =============================================
# COMMAND VALIDATION
# =============================================

check_command() {
    local cmd="$1"
    local install_hint="${2:-}"

    if ! command -v "$cmd" &> /dev/null; then
        print_error "Command not found: $cmd"
        if [[ -n "$install_hint" ]]; then
            print_info "Install with: $install_hint"
        fi
        return 1
    fi
    return 0
}

check_commands() {
    local missing=0
    local commands=("$@")

    for cmd in "${commands[@]}"; do
        if ! command -v "$cmd" &> /dev/null; then
            print_error "Required command not found: $cmd"
            missing=$((missing + 1))
        fi
    done

    return $missing
}

# =============================================
# FILE OPERATIONS
# =============================================

ensure_directory() {
    local dir="$1"

    if [[ ! -d "$dir" ]]; then
        print_step "Creating directory: $dir"
        mkdir -p "$dir"
        print_success "Directory created"
    fi
}

backup_file() {
    local file="$1"
    local backup="${file}.backup.$(date +%Y%m%d_%H%M%S)"

    if [[ -f "$file" ]]; then
        print_step "Creating backup: $backup"
        cp "$file" "$backup"
        print_success "Backup created"
        echo "$backup"
    fi
}

# =============================================
# GIT OPERATIONS
# =============================================

get_git_branch() {
    git rev-parse --abbrev-ref HEAD 2>/dev/null
}

get_git_commit() {
    git rev-parse --short HEAD 2>/dev/null
}

is_git_clean() {
    [[ -z "$(git status --porcelain)" ]]
}

# =============================================
# TIME OPERATIONS
# =============================================

timestamp() {
    date +"%Y-%m-%d %H:%M:%S"
}

# =============================================
# URL OPERATIONS
# =============================================

url_encode() {
    local string="$1"
    echo "$string" | jq -sRr @uri
}

# =============================================
# VALIDATION HELPERS
# =============================================

validate_env_var() {
    local var_name="$1"
    local var_value="${!var_name}"

    if [[ -z "$var_value" ]]; then
        print_error "Required environment variable not set: $var_name"
        return 1
    fi
    return 0
}

validate_env_vars() {
    local missing=0
    local vars=("$@")

    for var in "${vars[@]}"; do
        if ! validate_env_var "$var"; then
            missing=$((missing + 1))
        fi
    done

    return $missing
}

# =============================================
# CONFIRMATION PROMPTS
# =============================================

confirm_action() {
    local prompt="$1"
    local required_input="${2:-yes}"

    echo -e "${YELLOW}${prompt}${NC}"
    echo -e "${YELLOW}Type '${required_input}' to confirm: ${NC}"
    read -r confirmation

    if [[ "$confirmation" != "$required_input" ]]; then
        print_error "Confirmation failed. Expected '${required_input}', got '${confirmation}'"
        return 1
    fi

    print_success "Confirmed"
    return 0
}

# =============================================
# NETWORK OPERATIONS
# =============================================

test_url() {
    local url="$1"
    local expected_status="${2:-200}"

    local status
    status=$(curl -s -o /dev/null -w "%{http_code}" "$url" 2>/dev/null)

    if [[ "$status" == "$expected_status" ]]; then
        return 0
    else
        print_error "URL test failed: $url (expected $expected_status, got $status)"
        return 1
    fi
}

# =============================================
# CLEANUP HANDLERS
# =============================================

cleanup_on_error() {
    local exit_code=$?
    if [[ $exit_code -ne 0 ]]; then
        print_error "Script failed with exit code: $exit_code"
    fi
}

# Set trap for cleanup
trap cleanup_on_error EXIT

# =============================================
# SCRIPT UTILITIES
# =============================================

get_script_dir() {
    cd "$(dirname "${BASH_SOURCE[0]}")" && pwd
}

get_project_root() {
    git rev-parse --show-toplevel 2>/dev/null || pwd
}

# =============================================
# EXPORT FUNCTIONS
# =============================================

# Make all functions available to sourcing scripts
export -f print_header
export -f print_step
export -f print_success
export -f print_error
export -f print_warning
export -f print_info
export -f load_environment
export -f check_command
export -f check_commands
export -f ensure_directory
export -f backup_file
export -f get_git_branch
export -f get_git_commit
export -f is_git_clean
export -f timestamp
export -f url_encode
export -f validate_env_var
export -f validate_env_vars
export -f confirm_action
export -f test_url
export -f get_script_dir
export -f get_project_root
