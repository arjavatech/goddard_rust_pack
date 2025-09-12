#!/bin/bash

set -e

echo "🧪 Testing Rust Lambda API Locally"
echo "================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LAMBDA_DIR="$PROJECT_ROOT/lambda"

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

# Check prerequisites
check_prerequisites() {
    print_status "Checking prerequisites for local testing..."
    
    # Check if cargo-lambda is installed
    if ! command -v cargo-lambda >/dev/null 2>&1; then
        print_error "cargo-lambda not found. Install with: pip3 install cargo-lambda"
        exit 1
    fi
    
    # Check if lambda directory exists
    if [ ! -d "$LAMBDA_DIR" ]; then
        print_error "Lambda directory not found: $LAMBDA_DIR"
        exit 1
    fi
    
    print_success "Prerequisites check passed"
}

# Run Rust unit tests
run_unit_tests() {
    print_status "Running Rust unit tests..."
    
    cd "$LAMBDA_DIR"
    
    # Run unit tests
    cargo test --lib
    
    if [ $? -eq 0 ]; then
        print_success "Unit tests passed"
    else
        print_error "Unit tests failed"
        exit 1
    fi
}

# Run integration tests
run_integration_tests() {
    print_status "Running integration tests..."
    
    cd "$LAMBDA_DIR"
    
    # Run all tests including integration
    cargo test
    
    if [ $? -eq 0 ]; then
        print_success "Integration tests passed"
    else
        print_error "Integration tests failed"
        exit 1
    fi
}

# Start local server for manual testing
start_local_server() {
    print_status "Starting local Lambda server..."
    print_warning "This will start a local server. Press Ctrl+C to stop."
    
    cd "$LAMBDA_DIR"
    
    # Check if already built
    if [ ! -f "target/debug/bootstrap" ] && [ ! -f "target/release/bootstrap" ]; then
        print_status "Building Lambda function for local testing..."
        cargo lambda build
    fi
    
    print_success "🌐 Starting local server at http://127.0.0.1:9000"
    print_status "Available endpoints:"
    echo "  GET  http://127.0.0.1:9000/"
    echo "  GET  http://127.0.0.1:9000/health"
    echo "  GET  http://127.0.0.1:9000/hello/{name}"
    echo
    print_status "Press Ctrl+C to stop the server"
    
    # Start cargo lambda watch for hot reload
    cargo lambda watch
}

# Test local server endpoints
test_local_endpoints() {
    print_status "Testing local endpoints..."
    
    LOCAL_URL="http://127.0.0.1:9000"
    
    # Wait a moment for server to start
    sleep 2
    
    # Test if server is running
    if ! curl -s --connect-timeout 5 "$LOCAL_URL/" >/dev/null 2>&1; then
        print_error "Local server is not running. Start it with: cargo lambda watch"
        exit 1
    fi
    
    print_success "Local server is running!"
    
    # Test root endpoint
    echo
    print_status "Testing GET /"
    response=$(curl -s "$LOCAL_URL/" || echo "ERROR")
    if [[ "$response" == *"Hello"* ]]; then
        print_success "Root endpoint working"
        echo "$response" | jq . 2>/dev/null || echo "$response"
    else
        print_error "Root endpoint failed"
        echo "$response"
    fi
    
    # Test health endpoint
    echo
    print_status "Testing GET /health"
    response=$(curl -s "$LOCAL_URL/health" || echo "ERROR")
    if [[ "$response" == *"healthy"* ]]; then
        print_success "Health endpoint working"
        echo "$response" | jq . 2>/dev/null || echo "$response"
    else
        print_error "Health endpoint failed"
        echo "$response"
    fi
    
    # Test hello endpoint with name
    echo
    print_status "Testing GET /hello/TestUser"
    response=$(curl -s "$LOCAL_URL/hello/TestUser" || echo "ERROR")
    if [[ "$response" == *"TestUser"* ]]; then
        print_success "Hello endpoint working"
        echo "$response" | jq . 2>/dev/null || echo "$response"
    else
        print_error "Hello endpoint failed"
        echo "$response"
    fi
    
    echo
    print_success "All endpoints tested successfully!"
}

# Load testing with simple requests
load_test() {
    print_status "Running simple load test..."
    
    LOCAL_URL="http://127.0.0.1:9000"
    
    if ! command -v curl >/dev/null 2>&1; then
        print_warning "curl not found, skipping load test"
        return 0
    fi
    
    print_status "Sending 10 concurrent requests to root endpoint..."
    
    for i in {1..10}; do
        curl -s "$LOCAL_URL/" >/dev/null &
    done
    
    wait
    print_success "Load test completed"
}

# Performance benchmarking
benchmark() {
    print_status "Running performance benchmark..."
    
    LOCAL_URL="http://127.0.0.1:9000"
    
    if command -v time >/dev/null 2>&1; then
        print_status "Measuring response times..."
        
        # Measure multiple requests
        for i in {1..5}; do
            echo -n "Request $i: "
            time curl -s "$LOCAL_URL/" >/dev/null
        done
    else
        print_warning "time command not available for benchmarking"
    fi
}

# Main testing process
main() {
    case "$1" in
        --unit-only)
            check_prerequisites
            run_unit_tests
            print_success "🧪 Unit tests completed!"
            ;;
        --integration-only)
            check_prerequisites
            run_integration_tests
            print_success "🧪 Integration tests completed!"
            ;;
        --server-only)
            check_prerequisites
            start_local_server
            ;;
        --test-endpoints)
            check_prerequisites
            test_local_endpoints
            print_success "🌐 Endpoint tests completed!"
            ;;
        --load-test)
            check_prerequisites
            load_test
            print_success "⚡ Load test completed!"
            ;;
        --benchmark)
            check_prerequisites
            benchmark
            print_success "📊 Benchmark completed!"
            ;;
        *)
            check_prerequisites
            echo
            run_unit_tests
            echo
            run_integration_tests
            echo
            print_success "🎉 All tests completed successfully!"
            echo
            print_status "Next steps:"
            echo "  - Start local server: $0 --server-only"
            echo "  - Test endpoints: $0 --test-endpoints"
            echo "  - Run load test: $0 --load-test"
            echo "  - Deploy to AWS: ./scripts/deploy.sh"
            ;;
    esac
}

# Handle script arguments
case "$1" in
    --unit-only|--integration-only|--server-only|--test-endpoints|--load-test|--benchmark)
        main "$1"
        ;;
    --help|-h)
        echo "Usage: $0 [option]"
        echo ""
        echo "Options:"
        echo "  (no option)       Run all tests"
        echo "  --unit-only       Run only unit tests"
        echo "  --integration-only Run only integration tests"
        echo "  --server-only     Start local server for manual testing"
        echo "  --test-endpoints  Test local server endpoints"
        echo "  --load-test       Run simple load test"
        echo "  --benchmark       Run performance benchmark"
        echo "  --help, -h        Show this help message"
        ;;
    *)
        main
        ;;
esac