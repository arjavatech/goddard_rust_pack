#!/bin/bash

# Generalized Lambda Environment Variable Pusher
# Usage: LAMBDA_FUNCTION_NAME=goddard-dev ENV_FILE=.env.dev AWS_PROFILE=Arjava AWS_REGION=us-west-1 ./scripts/set-lambda-env.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration from environment variables with defaults
LAMBDA_FUNCTION_NAME="${LAMBDA_FUNCTION_NAME:-goddard-dev}"
ENV_FILE="${ENV_FILE:-.env.dev}"
AWS_PROFILE="${AWS_PROFILE:-Arjava}"
AWS_REGION="${AWS_REGION:-us-west-1}"

echo -e "${BLUE}⚙️  Pushing Environment Variables to Lambda${NC}"
echo -e "${BLUE}===========================================${NC}"
echo -e "Lambda Function: ${YELLOW}${LAMBDA_FUNCTION_NAME}${NC}"
echo -e "Env File:        ${YELLOW}${ENV_FILE}${NC}"
echo -e "AWS Profile:     ${YELLOW}${AWS_PROFILE}${NC}"
echo -e "AWS Region:      ${YELLOW}${AWS_REGION}${NC}"
echo ""

# Check if env file exists
if [[ ! -f "$ENV_FILE" ]]; then
    echo -e "${RED}❌ Error: $ENV_FILE not found!${NC}"
    exit 1
fi

# Check AWS CLI
if ! command -v aws &> /dev/null; then
    echo -e "${RED}❌ Error: AWS CLI not found!${NC}"
    exit 1
fi

# AWS Lambda reserved environment variables that cannot be modified
RESERVED_VARS=("AWS_REGION" "AWS_DEFAULT_REGION" "AWS_PROFILE" "_AWS_XRAY_TRACE_ID" "AWS_XRAY_CONTEXT_MISSING" "AWS_XRAY_DEBUG_MODE" "AWS_LAMBDA_FUNCTION_NAME" "AWS_LAMBDA_FUNCTION_MEMORY_SIZE" "AWS_LAMBDA_FUNCTION_VERSION" "AWS_LAMBDA_LOG_GROUP_NAME" "AWS_LAMBDA_LOG_STREAM_NAME" "AWS_LAMBDA_RUNTIME_API" "LAMBDA_TASK_ROOT" "LAMBDA_RUNTIME_DIR" "_HANDLER" "AWS_EXECUTION_ENV" "TZ")

# Convert .env file to JSON format for AWS Lambda
# Lambda limits the combined size of all environment variable keys and values to
# 4 KB. Keep local/Fly-only configuration in .env, but deploy only the variables
# the Rust Lambda reads at runtime. The final occurrence of a duplicate key wins,
# matching normal .env loading behaviour.
LAMBDA_RUNTIME_KEYS="|API_BASE_URL|CORS_ORIGINS|DATABASE_URL|EMAIL_FROM|EMAIL_PROVIDER|SMTP_HOST|SMTP_PORT|SMTP_USER|SMTP_PASS|ZEPTOMAIL_SEND_MAIL_TOKEN|FCM_PROJECT_ID|FCM_CLIENT_EMAIL|FCM_PRIVATE_KEY|FILLOUT_API_BASE_URL|FILLOUT_API_KEY|JWT_SECRET|LOG_LEVEL|OWNER_API_KEY|PARENT_DASHBOARD_URL|PORT|RUST_LOG|S3_UPLOAD_BUCKET|S3_BASE_URL|SUPABASE_URL|SUPABASE_ANON_KEY|SUPABASE_SERVICE_ROLE_KEY|TAP_TIME_API_URL|TAP_TIME_CONNECTION_ENCRYPTION_KEY|"

runtime_env_lines() {
    awk -v allowed="$LAMBDA_RUNTIME_KEYS" '
        !/^[[:space:]]*#/ && NF {
            equals = index($0, "=")
            if (equals == 0) next
            key = substr($0, 1, equals - 1)
            value = substr($0, equals + 1)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", key)
            if (index(allowed, "|" key "|") == 0) next
            if (!(key in seen)) order[++count] = key
            seen[key] = 1
            values[key] = value
        }
        END {
            for (i = 1; i <= count; i++) {
                key = order[i]
                print key "=" values[key]
            }
        }
    ' "$ENV_FILE"

}

convert_env_to_lambda_json() {
    local json_content='{"Variables":{'
    local first=true

    while IFS='=' read -r key value || [[ -n "$key" ]]; do
        # Skip comments and empty lines
        [[ "$key" =~ ^[[:space:]]*# ]] && continue
        [[ -z "$key" ]] && continue

        # Trim leading/trailing whitespace WITHOUT shell-quote interpretation.
        # Previously `xargs` was used here, but xargs parses input as quoted
        # shell tokens — inside a "..." value (e.g. FCM_PRIVATE_KEY) it eats
        # backslash escapes, collapsing every \n to a literal `n` and corrupting
        # multi-line values on their way to the Lambda environment.
        key="${key#"${key%%[![:space:]]*}"}"
        key="${key%"${key##*[![:space:]]}"}"
        value="${value#"${value%%[![:space:]]*}"}"
        value="${value%"${value##*[![:space:]]}"}"

        # Skip if key is empty
        [[ -z "$key" ]] && continue

        # Skip AWS reserved environment variables
        local is_reserved=false
        for reserved_var in "${RESERVED_VARS[@]}"; do
            if [[ "$key" == "$reserved_var" ]]; then
                echo -e "${YELLOW}⚠️  Skipping reserved AWS variable: $key${NC}" >&2
                is_reserved=true
                break
            fi
        done
        [[ "$is_reserved" == true ]] && continue

        # Remove surrounding quotes from value if present
        value=$(echo "$value" | sed 's/^"//; s/"$//')

        # Escape special characters for JSON
        value=$(echo "$value" | sed 's/\\/\\\\/g; s/"/\\"/g')

        if [[ "$first" == false ]]; then
            json_content+=","
        fi

        json_content+="\"$key\":\"$value\""
        first=false

    done < <(runtime_env_lines)

    json_content+='}}'
    echo "$json_content"
}

echo -e "${BLUE}📋 Processing environment variables from $ENV_FILE...${NC}"

ENV_JSON=$(convert_env_to_lambda_json)
echo "$ENV_JSON" > /tmp/lambda_env_$$.json

VAR_COUNT=$(echo "$ENV_JSON" | jq '.Variables | keys | length' 2>/dev/null || echo "unknown")
echo -e "${GREEN}✅ Found ${VAR_COUNT} environment variables${NC}"
echo -e "${BLUE}🚀 Updating Lambda configuration...${NC}"

if AWS_PROFILE="$AWS_PROFILE" AWS_DEFAULT_REGION="$AWS_REGION" aws lambda update-function-configuration \
    --profile "$AWS_PROFILE" \
    --region "$AWS_REGION" \
    --function-name "$LAMBDA_FUNCTION_NAME" \
    --environment "file:///tmp/lambda_env_$$.json" \
    --output table 2>&1; then

    echo -e "${GREEN}✅ Environment variables deployed successfully!${NC}"

    echo -e "${BLUE}⏳ Waiting for Lambda function to update...${NC}"
    AWS_PROFILE="$AWS_PROFILE" AWS_DEFAULT_REGION="$AWS_REGION" aws lambda wait function-updated \
        --profile "$AWS_PROFILE" \
        --region "$AWS_REGION" \
        --function-name "$LAMBDA_FUNCTION_NAME"

    echo -e "${GREEN}✅ Lambda function updated and ready!${NC}"
else
    echo -e "${RED}❌ Failed to deploy environment variables!${NC}"
    rm -f /tmp/lambda_env_$$.json
    exit 1
fi

rm -f /tmp/lambda_env_$$.json
echo -e "${GREEN}🎉 Done! $LAMBDA_FUNCTION_NAME now has env vars from $ENV_FILE${NC}"
