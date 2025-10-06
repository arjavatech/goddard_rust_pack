#!/bin/bash
set -e

echo "🚀 Starting deployment process..."

# Run build script first
./scripts/build.sh

# Navigate to infrastructure directory
cd infrastructure

# Bootstrap CDK (only needed once per account/region)
echo "🔧 Bootstrapping CDK (if needed)..."
npx cdk bootstrap || true

# Synthesize CloudFormation template
echo "📋 Synthesizing CloudFormation template..."
npm run synth

# Deploy the stack
echo "🚀 Deploying to AWS..."
npm run deploy -- --require-approval never

echo "✅ Deployment complete!"
echo "🎉 Your Rust Lambda is now live on AWS!"

# Show outputs
echo ""
echo "📊 Stack Outputs:"
aws cloudformation describe-stacks \
    --stack-name GoddardProdStack \
    --profile ${AWS_PROFILE:-default} \
    --region us-west-1 \
    --query 'Stacks[0].Outputs[*].[OutputKey,OutputValue]' \
    --output table 2>/dev/null || echo "Run 'aws cloudformation describe-stacks --stack-name GoddardProdStack --profile ${AWS_PROFILE:-default} --region us-west-1' to see outputs"