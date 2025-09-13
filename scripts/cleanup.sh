#!/bin/bash

set -e

echo "🧹 Cleaning up Rust Lambda API Project"
echo "====================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LAMBDA_DIR="$PROJECT_ROOT/lambda"
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

# Clean Rust build artifacts
clean_rust() {
    print_status "Cleaning Rust build artifacts..."
    
    if [ -d "$LAMBDA_DIR" ]; then
        cd "$LAMBDA_DIR"
        
        if [ -f "Cargo.toml" ]; then
            cargo clean
            print_success "Rust build artifacts cleaned"
        else
            print_warning "No Cargo.toml found, skipping Rust cleanup"
        fi
        
        # Remove additional cargo-lambda artifacts
        if [ -d "target/lambda" ]; then
            rm -rf "target/lambda"
            print_status "Removed cargo-lambda artifacts"
        fi
    else
        print_warning "Lambda directory not found: $LAMBDA_DIR"
    fi
}

# Clean Node.js dependencies and build artifacts
clean_nodejs() {
    print_status "Cleaning Node.js artifacts..."
    
    if [ -d "$INFRASTRUCTURE_DIR" ]; then
        cd "$INFRASTRUCTURE_DIR"
        
        # Remove node_modules
        if [ -d "node_modules" ]; then
            rm -rf "node_modules"
            print_status "Removed node_modules"
        fi
        
        # Remove CDK build artifacts
        if [ -d "cdk.out" ]; then
            rm -rf "cdk.out"
            print_status "Removed cdk.out"
        fi
        
        # Remove TypeScript outDir (dist)
        if [ -d "dist" ]; then
            rm -rf "dist"
            print_status "Removed dist directory"
        fi
        
        # Remove compiled TypeScript outputs (preserve source TS in lib/ if any)
        find lib -type f -name "*.js" -delete 2>/dev/null || true
        find lib -type f -name "*.d.ts" -delete 2>/dev/null || true
        find lib -type f -name "*.js.map" -delete 2>/dev/null || true
        
        # Remove compiled JS files
        find . -name "*.js" -not -path "./node_modules/*" -delete 2>/dev/null || true
        find . -name "*.d.ts" -not -path "./node_modules/*" -delete 2>/dev/null || true
        find . -name "*.js.map" -not -path "./node_modules/*" -delete 2>/dev/null || true
        
        print_success "Node.js artifacts cleaned"
    else
        print_warning "Infrastructure directory not found: $INFRASTRUCTURE_DIR"
    fi
}

# Clean CDK context and cache
clean_cdk_context() {
    print_status "Cleaning CDK context and cache..."
    
    if [ -d "$INFRASTRUCTURE_DIR" ]; then
        cd "$INFRASTRUCTURE_DIR"
        
        # Remove CDK context file
        if [ -f "cdk.context.json" ]; then
            rm -f "cdk.context.json"
            print_status "Removed cdk.context.json"
        fi
    fi
    
    # Clean global CDK cache
    if [ -d "$HOME/.cdk" ]; then
        print_status "Cleaning global CDK cache..."
        rm -rf "$HOME/.cdk/cache" 2>/dev/null || true
        print_status "CDK cache cleaned"
    fi
}

# Destroy AWS infrastructure
destroy_infrastructure() {
    if [ "$1" != "--destroy-aws" ]; then
        print_warning "Skipping AWS infrastructure destruction"
        print_status "To destroy AWS resources, run: $0 --destroy-aws"
        return 0
    fi
    
    print_warning "🚨 DESTROYING AWS INFRASTRUCTURE 🚨"
    print_status "This will delete all AWS resources created by this project"
    
    if [ -d "$INFRASTRUCTURE_DIR" ]; then
        cd "$INFRASTRUCTURE_DIR"
        
        # Check if CDK CLI is available
        if ! command -v cdk >/dev/null 2>&1; then
            print_error "CDK CLI not found. Install with: npm install -g aws-cdk"
            return 1
        fi
        
        # Check AWS credentials
        if ! aws sts get-caller-identity >/dev/null 2>&1; then
            print_error "AWS credentials not configured. Run 'aws configure'"
            return 1
        fi
        
        # Confirm destruction
        echo
        read -p "Are you sure you want to destroy all AWS resources? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            print_status "Destroying CDK stack..."
            cdk destroy --force
            print_success "AWS infrastructure destroyed"
        else
            print_status "Infrastructure destruction cancelled"
        fi
    else
        print_error "Infrastructure directory not found: $INFRASTRUCTURE_DIR"
    fi
}

# Clean logs and temporary files
clean_logs() {
    print_status "Cleaning logs and temporary files..."
    
    # Remove common log files
    find "$PROJECT_ROOT" -name "*.log" -delete 2>/dev/null || true
    find "$PROJECT_ROOT" -name ".DS_Store" -delete 2>/dev/null || true
    find "$PROJECT_ROOT" -name "Thumbs.db" -delete 2>/dev/null || true
    
    # Remove editor temporary files
    find "$PROJECT_ROOT" -name "*~" -delete 2>/dev/null || true
    find "$PROJECT_ROOT" -name "*.swp" -delete 2>/dev/null || true
    find "$PROJECT_ROOT" -name "*.swo" -delete 2>/dev/null || true
    
    print_success "Logs and temporary files cleaned"
}

# Show disk space saved
show_space_saved() {
    print_status "Cleanup summary:"
    echo
    
    # Check if main directories exist and show their absence
    if [ ! -d "$LAMBDA_DIR/target" ]; then
        echo "  ✓ Rust target/ directory cleaned"
    fi
    
    if [ ! -d "$INFRASTRUCTURE_DIR/node_modules" ]; then
        echo "  ✓ Node.js node_modules/ cleaned"
    fi
    
    if [ ! -d "$INFRASTRUCTURE_DIR/cdk.out" ]; then
        echo "  ✓ CDK build artifacts cleaned"
    fi
    
    echo
    print_success "Project cleaned successfully!"
}

# Main cleanup process
main() {
    print_status "Starting cleanup process..."
    
    clean_rust
    echo
    
    clean_nodejs
    echo
    
    clean_cdk_context
    echo
    
    clean_logs
    echo
    
    destroy_infrastructure "$1"
    echo
    
    show_space_saved
    echo
    
    if [ "$1" = "--destroy-aws" ]; then
        print_success "🧹 Complete cleanup with AWS destruction completed!"
    else
        print_success "🧹 Local cleanup completed!"
        print_status "To also destroy AWS resources, run: $0 --destroy-aws"
    fi
}

# Handle script arguments
case "$1" in
    --destroy-aws)
        echo
        print_warning "This will clean local files AND destroy AWS infrastructure"
        main --destroy-aws
        ;;
    --local-only)
        echo
        print_status "Cleaning only local files (keeping AWS resources)"
        clean_rust
        echo
        clean_nodejs
        echo
        clean_cdk_context
        echo
        clean_logs
        echo
        show_space_saved
        ;;
    --help|-h)
        echo "Usage: $0 [--destroy-aws] [--local-only] [--help]"
        echo ""
        echo "Options:"
        echo "  (no option)     Clean local files only"
        echo "  --destroy-aws   Clean local files AND destroy AWS infrastructure"
        echo "  --local-only    Clean only local files (explicit)"
        echo "  --help, -h      Show this help message"
        echo ""
        echo "What gets cleaned:"
        echo "  - Rust target/ directory and build artifacts"
        echo "  - Node.js node_modules/ and build artifacts"
        echo "  - CDK cdk.out/ and context files"
        echo "  - Log files and editor temporaries"
        echo "  - AWS resources (only with --destroy-aws)"
        ;;
    *)
        main
        ;;
esac


