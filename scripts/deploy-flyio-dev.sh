#!/bin/bash
# Deploy to Fly.io Development Environment

set -euo pipefail

# Navigate to project root (one level up from scripts directory)
cd "$(dirname "$0")/.."

# Unset global Fly API token to force local auth from ~/.fly/config.yml
# This prevents conflicts in restricted network environments
unset FLY_API_TOKEN

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Configuration
FLY_APP_NAME="goddard"
FLY_CONFIG="lambda/goddard/fly.toml"
PROJECT_DIR="lambda/goddard"

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}🚀 Deploying to Fly.io Development${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Check flyctl is installed
if ! command -v fly &> /dev/null; then
    echo -e "${RED}✗ flyctl not found${NC}"
    echo -e "${BLUE}Install with: brew install flyctl${NC}"
    exit 1
fi

# Check authentication (optional - may be restricted)
echo -e "${BLUE}▶ Checking Fly.io authentication${NC}"
set +e  # Temporarily allow errors
AUTH_OUTPUT=$(fly auth whoami 2>&1)
AUTH_EXIT_CODE=$?
set -e  # Re-enable exit on error
if [[ $AUTH_EXIT_CODE -ne 0 ]]; then
    echo -e "${YELLOW}⚠ Could not verify Fly.io authentication${NC}"
    echo -e "${BLUE}ℹ  This may be due to network restrictions${NC}"
    echo -e "${BLUE}ℹ  Deployment will use local auth from ~/.fly/config.yml${NC}"
    echo -e "${BLUE}ℹ  If deployment fails, try: fly auth login${NC}"
else
    echo -e "${GREEN}✓ Authenticated as: $(echo "$AUTH_OUTPUT" | head -n 1)${NC}"
fi

# Load environment
if [[ -f ".env.dev" ]]; then
    echo -e "${BLUE}▶ Loading .env.dev${NC}"
    set -a
    source .env.dev
    set +a
    echo -e "${GREEN}✓ Environment loaded${NC}"
fi

# Deploy
echo ""
echo -e "${BLUE}▶ Deploying to Fly.io${NC}"
echo -e "${BLUE}  App: ${FLY_APP_NAME}${NC}"
echo -e "${BLUE}  Config: ${FLY_CONFIG}${NC}"
echo ""

cd "$PROJECT_DIR"

if fly deploy --app "$FLY_APP_NAME" --config fly.toml; then
    echo ""
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}✨ Deployment successful!${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "${BLUE}🔗 URL: https://${FLY_APP_NAME}.fly.dev${NC}"
    echo -e "${BLUE}📊 Status: fly status --app ${FLY_APP_NAME}${NC}"
    echo -e "${BLUE}📝 Logs: fly logs --app ${FLY_APP_NAME}${NC}"
    echo ""
else
    echo ""
    echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${RED}❌ Deployment failed${NC}"
    echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "${BLUE}Check logs: fly logs --app ${FLY_APP_NAME}${NC}"
    exit 1
fi
