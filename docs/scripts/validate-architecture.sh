#!/bin/bash

# Architecture Validation Script for Lambda ARM64 Build
# Validates that both binary and CDK configuration are set for ARM64

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LAMBDA_DIR="$PROJECT_ROOT/lambda/hello-world"
INFRASTRUCTURE_DIR="$PROJECT_ROOT/infrastructure"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

echo "🔍 Validating Lambda ARM64 Architecture Configuration"
echo "=================================================="

# Validate binary architecture
validate_binary() {
    print_status "Checking Lambda binary architecture..."
    
    BINARY_PATH="$LAMBDA_DIR/target/lambda/hello-world/bootstrap"
    
    if [ ! -f "$BINARY_PATH" ]; then
        print_error "Bootstrap binary not found at: $BINARY_PATH"
        print_error "Run 'make build' first"
        return 1
    fi
    
    if command -v file >/dev/null 2>&1; then
        ARCH_INFO=$(file "$BINARY_PATH")
        echo "Binary info: $ARCH_INFO"
        
        if echo "$ARCH_INFO" | grep -q "aarch64\|ARM"; then
            print_success "✅ Binary compiled for ARM64/aarch64"
            return 0
        else
            print_error "❌ Binary NOT compiled for ARM64"
            print_error "Expected: ARM aarch64, Found: $ARCH_INFO"
            return 1
        fi
    else
        print_warning "Cannot verify - 'file' command not available"
        return 1
    fi
}

# Validate CDK configuration
validate_cdk() {
    print_status "Checking CDK Lambda architecture configuration..."
    
    if [ ! -d "$INFRASTRUCTURE_DIR/lib" ]; then
        print_error "CDK lib directory not found. Run 'make build' first"
        return 1
    fi
    
    cd "$INFRASTRUCTURE_DIR"
    
    # Check TypeScript source
    if [ -f "lib/rust-lambda-stack.ts" ]; then
        if grep -q "Architecture\.ARM_64" lib/rust-lambda-stack.ts; then
            print_success "✅ CDK TypeScript configured for ARM64"
        else
            print_error "❌ CDK TypeScript NOT configured for ARM64"
            return 1
        fi
    fi
    
    # Check compiled JavaScript
    if [ -f "lib/lib/rust-lambda-stack.js" ]; then
        if grep -q "Architecture\.ARM_64" lib/lib/rust-lambda-stack.js; then
            print_success "✅ CDK JavaScript compiled for ARM64"
        else
            print_error "❌ CDK JavaScript NOT compiled for ARM64"
            return 1
        fi
    fi
    
    return 0
}

# Validate runtime compatibility
validate_runtime() {
    print_status "Checking runtime compatibility..."
    
    cd "$INFRASTRUCTURE_DIR"
    
    if grep -q "PROVIDED_AL2023" lib/rust-lambda-stack.ts 2>/dev/null; then
        print_success "✅ Using provided.al2023 runtime (supports ARM64)"
    elif grep -q "PROVIDED_AL2" lib/rust-lambda-stack.ts 2>/dev/null; then
        print_success "✅ Using provided.al2 runtime (supports ARM64)"
    else
        print_warning "⚠️  Could not verify runtime supports ARM64"
    fi
}

# Show performance benefits
show_benefits() {
    print_status "ARM64 Benefits:"
    echo "  • 20% lower duration charges vs x86_64"
    echo "  • Up to 34% better price performance"
    echo "  • Up to 19% better performance for compute workloads"
    echo "  • Improved encryption and ML inference performance"
    echo ""
    print_status "AWS Documentation:"
    echo "  https://aws.amazon.com/blogs/compute/migrating-aws-lambda-functions-to-arm-based-aws-graviton2-processors/"
}

# Main validation
main() {
    VALIDATION_PASSED=true
    
    echo
    validate_binary || VALIDATION_PASSED=false
    echo
    
    validate_cdk || VALIDATION_PASSED=false
    echo
    
    validate_runtime
    echo
    
    if [ "$VALIDATION_PASSED" = true ]; then
        print_success "🎉 All ARM64 architecture validations passed!"
        echo
        show_benefits
    else
        print_error "❌ Architecture validation failed"
        echo
        print_status "To fix:"
        echo "  1. Build with: make build"
        echo "  2. Ensure CDK uses: Architecture.ARM_64"
        echo "  3. Use runtime: provided.al2023"
        exit 1
    fi
}

main "$@"