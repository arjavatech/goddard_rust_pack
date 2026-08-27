#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

AWS_PROFILE="arjava"
AWS_REGION="us-west-1"
STACK_NAME="GoddardDevStack"
LAMBDA_FUNCTION_NAME="goddard-dev"
NOTIFICATION_WORKER_FUNCTION_NAME="goddard-dev-notification-push-worker"
ENV_FILE=".env.dev"

echo -e "${BLUE}🚀 Deploying Goddard Dev to AWS Lambda${NC}"
echo -e "${BLUE}======================================${NC}"
echo -e "Stack:    ${YELLOW}${STACK_NAME}${NC}"
echo -e "Lambda:   ${YELLOW}${LAMBDA_FUNCTION_NAME}${NC}"
echo -e "Profile:  ${YELLOW}${AWS_PROFILE}${NC}"
echo -e "Region:   ${YELLOW}${AWS_REGION}${NC}"
echo -e "Env file: ${YELLOW}${ENV_FILE}${NC}"
echo ""

# Build binary
echo -e "${BLUE}🔨 Building Rust binary...${NC}"
./scripts/build.sh

# Bootstrap CDK (idempotent)
echo -e "${BLUE}🔧 Bootstrapping CDK (if needed)...${NC}"
cd infrastructure
AWS_PROFILE=$AWS_PROFILE npx cdk bootstrap --profile $AWS_PROFILE --region $AWS_REGION || true

# Deploy only dev stack
echo -e "${BLUE}📦 Deploying $STACK_NAME...${NC}"
AWS_PROFILE=$AWS_PROFILE npx cdk deploy $STACK_NAME \
    --profile $AWS_PROFILE \
    --require-approval never \
    --region $AWS_REGION
cd ..

# Set env vars
echo -e "${BLUE}⚙️  Pushing environment variables...${NC}"
LAMBDA_FUNCTION_NAME=$LAMBDA_FUNCTION_NAME \
    ENV_FILE=$ENV_FILE \
    AWS_PROFILE=$AWS_PROFILE \
    AWS_REGION=$AWS_REGION \
    ./scripts/set-lambda-env.sh

LAMBDA_FUNCTION_NAME=$NOTIFICATION_WORKER_FUNCTION_NAME \
    ENV_SCOPE=notification_worker \
    ENV_FILE=$ENV_FILE \
    AWS_PROFILE=$AWS_PROFILE \
    AWS_REGION=$AWS_REGION \
    ./scripts/set-lambda-env.sh

# Print API URL
echo ""
echo -e "${BLUE}📊 Stack Outputs:${NC}"
API_URL=$(aws cloudformation describe-stacks \
    --stack-name $STACK_NAME \
    --profile $AWS_PROFILE \
    --region $AWS_REGION \
    --query 'Stacks[0].Outputs[?OutputKey==`ApiUrl`].OutputValue' \
    --output text 2>/dev/null || echo "")

if [[ -n "$API_URL" ]]; then
    echo -e "${GREEN}✅ Dev API URL: ${YELLOW}${API_URL}${NC}"
    echo -e "${BLUE}💡 Smoke test: ${NC}curl ${API_URL}health"
else
    echo -e "${YELLOW}⚠️  Could not retrieve API URL. Check CloudFormation outputs.${NC}"
fi

echo ""
echo -e "${GREEN}🎉 Dev deployment complete!${NC}"
