#!/bin/bash
# Deploy to Fly.io Production Environment

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
FLY_APP_NAME="goddard-falling-surf-1798"
FLY_CONFIG="lambda/goddard/fly.toml.production"
PROJECT_DIR="lambda/goddard"

echo ""
echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${RED}⚠️  PRODUCTION DEPLOYMENT WARNING${NC}"
echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}You are about to deploy to PRODUCTION:${NC}"
echo ""
echo -e "${BLUE}  App: ${FLY_APP_NAME}${NC}"
echo -e "${BLUE}  Config: ${FLY_CONFIG}${NC}"
echo -e "${BLUE}  URL: https://${FLY_APP_NAME}.fly.dev${NC}"
echo ""
echo -e "${RED}This will affect live users!${NC}"
echo ""

# Confirmation gate
echo -e "${YELLOW}Type 'YES' (in capitals) to confirm deployment: ${NC}"
read -r confirmation

if [[ "$confirmation" != "YES" ]]; then
    echo ""
    echo -e "${RED}✗ Deployment cancelled${NC}"
    echo -e "${BLUE}You typed: '$confirmation'${NC}"
    echo -e "${BLUE}Required: 'YES'${NC}"
    echo ""
    exit 1
fi

echo ""
echo -e "${GREEN}✓ Confirmed${NC}"
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
if [[ -f ".env.production" ]]; then
    echo -e "${BLUE}▶ Loading .env.production${NC}"
    set -a
    source .env.production
    set +a
    echo -e "${GREEN}✓ Environment loaded${NC}"
fi

# Create backup reference point
echo -e "${BLUE}▶ Recording current deployment${NC}"
fly releases --app "$FLY_APP_NAME" --limit 1 2>/dev/null | tail -n 1 || echo "  (no previous releases)"

# Deploy
echo ""
echo -e "${BLUE}▶ Deploying to Fly.io Production${NC}"
echo ""

cd "$PROJECT_DIR"

if fly deploy --app "$FLY_APP_NAME" --config fly.toml.production; then
    echo ""
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}✨ Production deployment successful!${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "${BLUE}🔗 URL: https://${FLY_APP_NAME}.fly.dev${NC}"
    echo -e "${BLUE}📊 Status: fly status --app ${FLY_APP_NAME}${NC}"
    echo -e "${BLUE}📝 Logs: fly logs --app ${FLY_APP_NAME}${NC}"
    echo -e "${BLUE}📈 Dashboard: fly dashboard --app ${FLY_APP_NAME}${NC}"
    echo ""
    echo -e "${YELLOW}⏪ Rollback: fly releases --app ${FLY_APP_NAME}${NC}"
    echo ""
else
    echo ""
    echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${RED}❌ Production deployment failed${NC}"
    echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "${BLUE}Check logs: fly logs --app ${FLY_APP_NAME}${NC}"
    echo -e "${YELLOW}Rollback: fly releases --app ${FLY_APP_NAME}${NC}"
    exit 1
fi
