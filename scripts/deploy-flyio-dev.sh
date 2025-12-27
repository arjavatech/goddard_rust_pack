#!/bin/bash
set -e

echo "🚀 Deploying to Fly.io Development (goddard)..."

cd "$(dirname "$0")/../lambda/goddard"

# Temporarily unset global FLY_API_TOKEN to use local auth from ~/.fly/config.yml
unset FLY_API_TOKEN

# Verify Fly.io authentication
if ! fly auth whoami > /dev/null 2>&1; then
  echo "❌ Not authenticated with Fly.io"
  echo "Run: fly auth login"
  exit 1
fi

# Deploy to dev app
fly deploy --config fly.toml --app goddard

echo "✅ Development deployment complete!"
echo "📊 View logs: fly logs --app goddard"
echo "🌐 App URL: https://goddard.fly.dev"
