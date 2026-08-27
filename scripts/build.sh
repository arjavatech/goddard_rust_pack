#!/bin/bash
set -e

echo "🦀 Building Rust Lambda function..."

# Navigate to lambda directory
cd lambda/goddard

# Install cargo-lambda if not already installed
if ! command -v cargo-lambda &> /dev/null; then
    echo "📦 Installing cargo-lambda..."
    pip3 install cargo-lambda
fi

# Build the Lambda function
echo "🔨 Building Lambda function for ARM64..."
cargo lambda build --release --arm64 --bins

echo "✅ Rust Lambda build complete!"

# Navigate to infrastructure directory
cd ../../infrastructure

# Install CDK dependencies
echo "📦 Installing CDK dependencies..."
npm install

# Build TypeScript
echo "🔨 Building CDK TypeScript..."
npm run build

echo "✅ Infrastructure build complete!"
echo "🎉 Build process finished successfully!"
