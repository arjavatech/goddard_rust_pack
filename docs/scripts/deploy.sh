#!/bin/bash

set -e

echo "🚀 Deploying Rust Lambda API to AWS"
echo "=================================="

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

# Check prerequisites
check_prerequisites() {
    print_status "Checking prerequisites..."
    
    # Check AWS CLI
    if ! command -v aws >/dev/null 2>&1; then
        print_error "AWS CLI not found. Please install AWS CLI v2"
        exit 1
    fi
    
    # Check AWS credentials
    if ! aws sts get-caller-identity >/dev/null 2>&1; then
        print_error "AWS credentials not configured. Run 'aws configure'"
        exit 1
    fi
    
    # Check CDK CLI
    if ! command -v cdk >/dev/null 2>&1; then
        print_error "CDK CLI not found. Install with: npm install -g aws-cdk"
        exit 1
    fi
    
    # Check if build artifacts exist
    if [ ! -d "$LAMBDA_DIR/target" ]; then
        print_warning "Lambda not built. Running build first..."
        "$PROJECT_ROOT/scripts/build.sh" --skip-tests
    fi
    
    print_success "Prerequisites check passed"
}

# Bootstrap CDK (if needed)
bootstrap_cdk() {
    print_status "Checking CDK bootstrap status..."
    
    cd "$INFRASTRUCTURE_DIR"
    
    # Get AWS account and region
    ACCOUNT=$(aws sts get-caller-identity --query Account --output text)
    REGION=$(aws configure get region)
    
    if [ -z "$REGION" ]; then
        REGION="us-east-1"
        print_warning "No default region set, using us-east-1"
    fi
    
    print_status "Deploying to Account: $ACCOUNT, Region: $REGION"
    
    # Check if already bootstrapped
    if ! aws cloudformation describe-stacks --stack-name CDKToolkit --region "$REGION" >/dev/null 2>&1; then
        print_status "Bootstrapping CDK for account $ACCOUNT in region $REGION..."
        cdk bootstrap "aws://$ACCOUNT/$REGION"
        print_success "CDK bootstrap completed"
    else
        print_status "CDK already bootstrapped"
    fi
}

# Deploy the stack
deploy_stack() {
    print_status "Deploying CDK stack..."
    
    cd "$INFRASTRUCTURE_DIR"
    
    # Build the CDK project first
    npm run build
    
    # Deploy with confirmation skip for automation
    if [ "$1" = "--auto-approve" ]; then
        print_status "Auto-approving deployment..."
        cdk deploy --require-approval never
    else
        print_status "Deploying with manual approval..."
        cdk deploy
    fi
    
    if [ $? -eq 0 ]; then
        print_success "Stack deployed successfully"
    else
        print_error "Stack deployment failed"
        exit 1
    fi
}

# Test the deployed API
test_deployment() {
    if [ "$1" = "--skip-test" ]; then
        print_warning "Skipping deployment test"
        return 0
    fi
    
    print_status "Testing deployed API..."
    
    cd "$INFRASTRUCTURE_DIR"
    
    # Get API URL from stack outputs
    API_URL=$(cdk output RustLambdaApiStack.ApiUrl 2>/dev/null || echo "")
    
    if [ -z "$API_URL" ]; then
        # Try alternative method to get API URL
        API_ID=$(aws apigateway get-rest-apis --query "items[?name=='Rust Lambda API'].id" --output text)
        REGION=$(aws configure get region || echo "us-east-1")
        API_URL="https://${API_ID}.execute-api.${REGION}.amazonaws.com/prod"
    fi
    
    if [ -n "$API_URL" ]; then
        print_status "Testing API at: $API_URL"
        
        # Test root endpoint
        echo
        print_status "Testing GET /"
        curl -s "$API_URL/" | jq . || print_warning "jq not available, showing raw response:"
        
        # Test health endpoint
        echo
        print_status "Testing GET /health"
        curl -s "$API_URL/health" | jq . || print_warning "jq not available"
        
        # Test hello endpoint with name
        echo
        print_status "Testing GET /hello/World"
        curl -s "$API_URL/hello/World" | jq . || print_warning "jq not available"
        
        echo
        print_success "API endpoints tested successfully"
        print_success "🌐 Your API is available at: $API_URL"
    else
        print_error "Could not determine API URL"
    fi
}

# Display useful information
show_info() {
    print_status "Deployment Information:"
    echo
    
    cd "$INFRASTRUCTURE_DIR"
    
    # Show stack outputs
    echo "Stack Outputs:"
    cdk output 2>/dev/null || print_warning "No outputs available"
    
    echo
    print_status "Useful Commands:"
    echo "  View logs: aws logs tail /aws/lambda/rust-hello-world-api --follow"
    echo "  Update stack: ./scripts/deploy.sh"
    echo "  Destroy stack: ./scripts/cleanup.sh"
    echo "  Test local: ./scripts/test-local.sh"
}

# Main deployment process
main() {
    print_status "Starting deployment..."
    
    check_prerequisites
    echo
    
    bootstrap_cdk
    echo
    
    deploy_stack "$1"
    echo
    
    test_deployment "$2"
    echo
    
    show_info
    echo
    
    print_success "🎉 Deployment completed successfully!"
}

# Handle script arguments
case "$1" in
    --auto-approve)
        main --auto-approve "$2"
        ;;
    --skip-test)
        main "" --skip-test
        ;;
    --help|-h)
        echo "Usage: $0 [--auto-approve] [--skip-test] [--help]"
        echo ""
        echo "Options:"
        echo "  --auto-approve  Skip CDK deployment approval prompts"
        echo "  --skip-test     Skip testing the deployed API"
        echo "  --help, -h      Show this help message"
        ;;
    *)
        main
        ;;
esac