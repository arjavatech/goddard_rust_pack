#!/bin/bash

set -e

echo "🚀 Building Rust Lambda API Project"
echo "=================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LAMBDA_DIR="$PROJECT_ROOT/lambda/goddard"
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

# Check if directories exist
check_directories() {
    print_status "Checking project structure..."
    
    if [ ! -d "$LAMBDA_DIR" ]; then
        print_error "Lambda directory not found: $LAMBDA_DIR"
        exit 1
    fi
    
    if [ ! -d "$INFRASTRUCTURE_DIR" ]; then
        print_error "Infrastructure directory not found: $INFRASTRUCTURE_DIR"
        exit 1
    fi
    
    print_success "Project structure verified"
}

# Build Rust Lambda function
build_lambda() {
    print_status "Building Rust Lambda function..."
    
    cd "$LAMBDA_DIR"
    
    # Check if Cargo.toml exists
    if [ ! -f "Cargo.toml" ]; then
        print_error "Cargo.toml not found in lambda directory"
        exit 1
    fi
    
    # Clean previous builds
    cargo clean
    
    # Build for AWS Lambda ARM64 architecture (using cargo-lambda)
    if command -v cargo-lambda >/dev/null 2>&1; then
        print_status "Using cargo-lambda for optimized ARM64 build..."
        cargo lambda build --release --arm64
    else
        print_error "cargo-lambda is required for ARM64 Lambda builds"
        print_error "Install with: cargo install cargo-lambda"
        print_error "Then run this script again"
        exit 1
    fi
    
    # Check if build was successful
    if [ $? -eq 0 ]; then
        print_success "Rust Lambda ARM64 build completed"
        
        # Validate the binary architecture
        if [ -f "target/lambda/goddard-backend/bootstrap" ]; then
            print_status "Validating binary architecture..."
            if command -v file >/dev/null 2>&1; then
                ARCH_INFO=$(file target/lambda/goddard-backend/bootstrap)
                if echo "$ARCH_INFO" | grep -q "aarch64\|ARM"; then
                    print_success "✅ Binary verified as ARM64/aarch64"
                else
                    print_warning "⚠️  Binary architecture: $ARCH_INFO"
                    print_warning "Expected ARM64/aarch64, but may still work"
                fi
            else
                print_warning "Cannot verify architecture - 'file' command not available"
            fi
        else
            print_error "Bootstrap binary not found at expected location"
            exit 1
        fi
    else
        print_error "Rust Lambda build failed"
        exit 1
    fi
}

# Build CDK infrastructure
build_infrastructure() {
    print_status "Building CDK infrastructure..."
    
    cd "$INFRASTRUCTURE_DIR"
    
    # Check if package.json exists
    if [ ! -f "package.json" ]; then
        print_error "package.json not found in infrastructure directory"
        exit 1
    fi
    
    # Install dependencies
    print_status "Installing npm dependencies..."
    npm install
    
    # Build TypeScript
    print_status "Compiling TypeScript..."
    npm run build
    
    # Check if build was successful
    if [ $? -eq 0 ]; then
        print_success "CDK infrastructure build completed"
        
        # Validate CDK configuration for ARM64
        print_status "Validating CDK Lambda architecture configuration..."
        if grep -q "Architecture\.ARM_64" lib/*.js 2>/dev/null; then
            print_success "✅ CDK configured for ARM64 architecture"
        elif grep -q "Architecture\.X86_64" lib/*.js 2>/dev/null; then
            print_error "❌ CDK configured for X86_64 but binary is ARM64"
            print_error "Update CDK stack to use Architecture.ARM_64"
            exit 1
        else
            print_warning "⚠️  Could not verify CDK architecture configuration"
        fi
    else
        print_error "CDK infrastructure build failed"
        exit 1
    fi
}

# Run tests
run_tests() {
    if [ "$1" = "--skip-tests" ]; then
        print_warning "Skipping tests"
        return 0
    fi
    
    print_status "Running tests..."
    
    # Test Rust Lambda
    cd "$LAMBDA_DIR"
    print_status "Running Rust tests..."
    cargo test
    
    if [ $? -ne 0 ]; then
        print_error "Rust tests failed"
        exit 1
    fi
    
    # Test CDK infrastructure
    cd "$INFRASTRUCTURE_DIR"
    print_status "Running CDK tests..."
    npm test
    
    if [ $? -ne 0 ]; then
        print_error "CDK tests failed"
        exit 1
    fi
    
    print_success "All tests passed"
}

# Main build process
main() {
    echo
    print_status "Starting build process..."
    
    check_directories
    echo
    
    build_lambda
    echo
    
    build_infrastructure
    echo
    
    run_tests "$1"
    echo
    
    print_success "🎉 Build completed successfully!"
    echo
    print_status "Next steps:"
    echo "  - Deploy: ./scripts/deploy.sh"
    echo "  - Test locally: ./scripts/test-local.sh"
    echo "  - Clean: make clean"
}

# Handle script arguments
case "$1" in
    --skip-tests)
        main --skip-tests
        ;;
    --help|-h)
        echo "Usage: $0 [--skip-tests] [--help]"
        echo ""
        echo "Options:"
        echo "  --skip-tests  Skip running tests during build"
        echo "  --help, -h    Show this help message"
        ;;
    *)
        main
        ;;
esac