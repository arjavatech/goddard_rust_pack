#!/bin/bash

# Deploy Environment Variables to AWS Lambda (Non-Interactive Version)
# This script automatically deploys without user confirmation

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
LAMBDA_FUNCTION_NAME="RustLambdaStack-GoddardLambdaC65E3A55-d4oUhnRvr8VQ"
ENV_FILE=".env"
AWS_PROFILE="${AWS_PROFILE:-goddard}"
AWS_REGION="us-west-2"

echo -e "${BLUE}🚀 Auto-Deploying Environment Variables to Lambda${NC}"
echo -e "${BLUE}============================================${NC}"
echo -e "Lambda Function: ${YELLOW}${LAMBDA_FUNCTION_NAME}${NC}"
echo -e "AWS Profile: ${YELLOW}${AWS_PROFILE}${NC}"
echo -e "Environment File: ${YELLOW}${ENV_FILE}${NC}"
echo ""

# Check if .env file exists
if [[ ! -f "$ENV_FILE" ]]; then
    echo -e "${RED}❌ Error: $ENV_FILE file not found!${NC}"
    echo -e "${YELLOW}💡 Please ensure you have a .env file in the project root${NC}"
    exit 1
fi

# Check if AWS CLI is installed and configured
if ! command -v aws &> /dev/null; then
    echo -e "${RED}❌ Error: AWS CLI not found!${NC}"
    echo -e "${YELLOW}💡 Please install AWS CLI first${NC}"
    exit 1
fi

# Check if the specified AWS profile exists
if ! aws configure list-profiles | grep -q "^${AWS_PROFILE}$"; then
    echo -e "${RED}❌ Error: AWS profile '${AWS_PROFILE}' not found!${NC}"
    echo -e "${YELLOW}💡 Available profiles:${NC}"
    aws configure list-profiles
    exit 1
fi

# Function to convert .env file to JSON format for AWS Lambda
convert_env_to_lambda_json() {
    local json_content='{"Variables":{'
    local first=true

    # AWS Lambda reserved environment variables that cannot be modified
    local reserved_vars=("AWS_REGION" "AWS_DEFAULT_REGION" "AWS_PROFILE" "_AWS_XRAY_TRACE_ID" "AWS_XRAY_CONTEXT_MISSING" "AWS_XRAY_DEBUG_MODE" "AWS_LAMBDA_FUNCTION_NAME" "AWS_LAMBDA_FUNCTION_MEMORY_SIZE" "AWS_LAMBDA_FUNCTION_VERSION" "AWS_LAMBDA_LOG_GROUP_NAME" "AWS_LAMBDA_LOG_STREAM_NAME" "AWS_LAMBDA_RUNTIME_API" "LAMBDA_TASK_ROOT" "LAMBDA_RUNTIME_DIR" "_HANDLER" "AWS_EXECUTION_ENV" "TZ")

    # Read .env file and convert to JSON
    while IFS='=' read -r key value || [[ -n "$key" ]]; do
        # Skip comments and empty lines
        [[ "$key" =~ ^[[:space:]]*# ]] && continue
        [[ -z "$key" ]] && continue

        # Remove leading/trailing whitespace
        key=$(echo "$key" | xargs)
        value=$(echo "$value" | xargs)

        # Skip if key is empty
        [[ -z "$key" ]] && continue

        # Skip AWS reserved environment variables
        local is_reserved=false
        for reserved_var in "${reserved_vars[@]}"; do
            if [[ "$key" == "$reserved_var" ]]; then
                echo -e "${YELLOW}⚠️  Skipping reserved AWS variable: $key${NC}" >&2
                is_reserved=true
                break
            fi
        done
        [[ "$is_reserved" == true ]] && continue

        # Remove quotes from value if present
        value=$(echo "$value" | sed 's/^"//; s/"$//')

        # Escape special characters for JSON
        value=$(echo "$value" | sed 's/\\/\\\\/g; s/"/\\"/g')

        # Add comma if not first entry
        if [[ "$first" == false ]]; then
            json_content+=","
        fi

        json_content+="\"$key\":\"$value\""
        first=false

    done < "$ENV_FILE"

    json_content+='}}'
    echo "$json_content"
}

echo -e "${BLUE}📋 Processing environment variables from $ENV_FILE...${NC}"

# Convert .env to JSON and save to temp file
ENV_JSON=$(convert_env_to_lambda_json)
echo "$ENV_JSON" > /tmp/lambda_env.json

# Count variables
VAR_COUNT=$(echo "$ENV_JSON" | jq '.Variables | keys | length' 2>/dev/null || echo "unknown")

echo -e "${GREEN}✅ Found ${VAR_COUNT} environment variables${NC}"
echo -e "${BLUE}🚀 Auto-deploying to AWS Lambda (no confirmation needed)...${NC}"
echo ""

# Deploy environment variables to Lambda using file input
echo -e "${BLUE}🔧 Running AWS Lambda update command...${NC}"
echo -e "${BLUE}Using AWS Profile: ${AWS_PROFILE}, Region: ${AWS_REGION}${NC}"
if AWS_PROFILE="$AWS_PROFILE" AWS_DEFAULT_REGION="$AWS_REGION" aws lambda update-function-configuration \
    --region "$AWS_REGION" \
    --function-name "$LAMBDA_FUNCTION_NAME" \
    --environment file:///tmp/lambda_env.json \
    --output table 2>&1; then

    echo -e "${GREEN}✅ Environment variables deployed successfully!${NC}"
    echo ""

    # Show confirmation
    echo -e "${BLUE}📊 Deployment Summary:${NC}"
    echo -e "  Function: ${YELLOW}$LAMBDA_FUNCTION_NAME${NC}"
    echo -e "  Variables: ${YELLOW}$VAR_COUNT${NC}"
    echo -e "  AWS Profile: ${YELLOW}$AWS_PROFILE${NC}"
    echo ""

    # Wait for Lambda to update
    echo -e "${BLUE}⏳ Waiting for Lambda function to update...${NC}"
    AWS_PROFILE="$AWS_PROFILE" AWS_DEFAULT_REGION="$AWS_REGION" aws lambda wait function-updated \
        --region "$AWS_REGION" \
        --function-name "$LAMBDA_FUNCTION_NAME"

    echo -e "${GREEN}✅ Lambda function updated successfully!${NC}"

    # Clean up temp file
    rm -f /tmp/lambda_env.json

    echo -e "${GREEN}🎉 Environment variable deployment completed!${NC}"
    echo -e "${BLUE}💡 Your Lambda function now has all environment variables from $ENV_FILE${NC}"

else
    echo -e "${RED}❌ Failed to deploy environment variables!${NC}"
    echo -e "${YELLOW}💡 Check your AWS credentials and permissions${NC}"
    rm -f /tmp/lambda_env.json
    exit 1
fi