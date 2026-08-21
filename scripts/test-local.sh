#!/bin/bash

# =============================================
# Goddard School Local Testing Script
# =============================================

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🏫 Goddard School Local Testing${NC}"
echo -e "${BLUE}================================${NC}"

# Check if cargo-lambda is installed
if ! command -v cargo-lambda &> /dev/null; then
    echo -e "${RED}❌ cargo-lambda not found${NC}"
    echo -e "${YELLOW}Installing cargo-lambda...${NC}"
    cargo install cargo-lambda
fi

echo -e "${YELLOW}🔨 Building Lambda function...${NC}"
cd lambda/goddard

# Load environment variables
if [ -f "../../.env" ]; then
    echo -e "${BLUE}📋 Loading environment variables...${NC}"
    # Preserve quoted values and values containing spaces (for example,
    # EMAIL_FROM="Goddard Schools <noreply@goddardschool.org>").
    set -a
    source ../../.env
    set +a
fi

# Build the Lambda function for local testing
cargo lambda build --release

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Build successful!${NC}"
else
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

echo -e "${YELLOW}🚀 Starting local Lambda server...${NC}"
echo -e "${BLUE}Server will be available at: http://localhost:9000${NC}"
echo -e "${BLUE}To test endpoints:${NC}"
echo -e "${GREEN}  GET  http://localhost:9000/health${NC}"
echo -e "${GREEN}  GET  http://localhost:9000/schools${NC}"
echo -e "${GREEN}  POST http://localhost:9000/schools${NC}"
echo -e "${GREEN}  GET  http://localhost:9000/users${NC}"
echo -e "${BLUE}Press Ctrl+C to stop the server${NC}"
echo ""

# Start the local Lambda server
cargo lambda start --release
