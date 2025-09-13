#!/bin/bash
set -e

echo "🧪 Starting local testing environment..."

cd lambda/goddard

# Install cargo-lambda if not already installed
if ! command -v cargo-lambda &> /dev/null; then
    echo "📦 Installing cargo-lambda..."
    pip3 install cargo-lambda
fi

# Start local Lambda runtime
echo "🚀 Starting cargo-lambda watch..."
echo "📡 Server will be available at http://localhost:9000"
echo ""
echo "Test endpoints:"
echo "  GET http://localhost:9000/"
echo "  GET http://localhost:9000/hello/YourName"
echo "  GET http://localhost:9000/health"
echo ""
echo "Press Ctrl+C to stop"

cargo lambda watch