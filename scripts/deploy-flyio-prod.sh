#!/bin/bash
set -e

echo "🚀 Deploying to Fly.io Production (goddard-falling-surf-1798)..."

# Confirmation prompt
read -p "⚠️  Deploy to PRODUCTION? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
  echo "❌ Deployment cancelled"
  exit 1
fi

cd "$(dirname "$0")/../lambda/goddard"

# Temporarily unset global FLY_API_TOKEN to use local auth from ~/.fly/config.yml
unset FLY_API_TOKEN

# Verify Fly.io authentication
if ! fly auth whoami > /dev/null 2>&1; then
  echo "❌ Not authenticated with Fly.io"
  echo "Run: fly auth login"
  exit 1
fi

# Deploy to production app
fly deploy --config fly.production.toml --app goddard-falling-surf-1798

echo "✅ Production deployment complete!"
echo "📊 View logs: fly logs --app goddard-falling-surf-1798"
echo "🌐 App URL: https://goddard-falling-surf-1798.fly.dev"
