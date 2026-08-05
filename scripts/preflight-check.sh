#!/bin/bash
# Standalone Preflight Check Script
# Run comprehensive validation before deployment
# Usage: ./scripts/preflight-check.sh [dev|production]

set -euo pipefail

# Navigate to script directory
cd "$(dirname "$0")"

# Navigate to project root
cd ..

# Source validation functions
source scripts/lib/common.sh
source scripts/lib/validators.sh

# Parse environment argument
ENV="${1:-dev}"

# Validate environment argument
if [[ ! "$ENV" =~ ^(dev|production)$ ]]; then
    print_error "Invalid environment: $ENV"
    print_info "Usage: $0 [dev|production]"
    exit 1
fi

# Run all preflight checks
run_all_preflight_checks "$ENV"
