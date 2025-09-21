#!/bin/bash

# Deploy Environment Variables to AWS Lambda
# This script reads from the consolidated .env file and sets them directly in Lambda

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

echo -e "${BLUE}🚀 Deploying Environment Variables to Lambda${NC}"
echo -e "${BLUE}======================================${NC}"
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
convert_env_to_json() {
    local json_vars="{"
    local first=true

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

        # Remove quotes from value if present
        value=$(echo "$value" | sed 's/^"//; s/"$//')

        # Escape special characters for JSON
        value=$(echo "$value" | sed 's/\\/\\\\/g; s/"/\\"/g')

        # Add comma if not first entry
        if [[ "$first" == false ]]; then
            json_vars+=","
        fi

        json_vars+="\"$key\":\"$value\""
        first=false

    done < "$ENV_FILE"

    json_vars+="}"
    echo "$json_vars"
}

echo -e "${BLUE}📋 Processing environment variables from $ENV_FILE...${NC}"

# Convert .env to JSON
ENV_JSON=$(convert_env_to_json)

# Count variables
VAR_COUNT=$(echo "$ENV_JSON" | jq 'keys | length' 2>/dev/null || echo "unknown")

echo -e "${GREEN}✅ Found ${VAR_COUNT} environment variables${NC}"
echo ""

# Show preview of variables (first 5)
echo -e "${BLUE}📝 Preview of variables to deploy:${NC}"
echo "$ENV_JSON" | jq -r 'to_entries | .[0:5] | .[] | "  \(.key) = \(.value)"' 2>/dev/null || {
    echo -e "${YELLOW}⚠️  Could not parse JSON preview (jq not available)${NC}"
}

if [[ "$VAR_COUNT" -gt 5 ]]; then
    echo -e "  ${YELLOW}... and $((VAR_COUNT - 5)) more variables${NC}"
fi
echo ""

# Confirm deployment
read -p "$(echo -e "${YELLOW}🤔 Do you want to deploy these environment variables to Lambda? [y/N]: ${NC}")" -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}⏹️  Deployment cancelled${NC}"
    exit 0
fi

echo ""
echo -e "${BLUE}🚀 Deploying to AWS Lambda...${NC}"

# Deploy environment variables to Lambda
if aws lambda update-function-configuration \
    --profile "$AWS_PROFILE" \
    --function-name "$LAMBDA_FUNCTION_NAME" \
    --environment "Variables=$ENV_JSON" \
    --output table > /dev/null 2>&1; then

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
    aws lambda wait function-updated \
        --profile "$AWS_PROFILE" \
        --function-name "$LAMBDA_FUNCTION_NAME"

    echo -e "${GREEN}✅ Lambda function updated successfully!${NC}"

    # Optional: Show current environment variables
    read -p "$(echo -e "${YELLOW}🔍 Show current Lambda environment variables? [y/N]: ${NC}")" -n 1 -r
    echo ""

    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo ""
        echo -e "${BLUE}📋 Current Lambda Environment Variables:${NC}"
        aws lambda get-function-configuration \
            --profile "$AWS_PROFILE" \
            --function-name "$LAMBDA_FUNCTION_NAME" \
            --query 'Environment.Variables' \
            --output table
    fi

else
    echo -e "${RED}❌ Failed to deploy environment variables!${NC}"
    echo -e "${YELLOW}💡 Check your AWS credentials and permissions${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}🎉 Environment variable deployment completed!${NC}"
echo -e "${BLUE}💡 Your Lambda function now has all environment variables from $ENV_FILE${NC}"