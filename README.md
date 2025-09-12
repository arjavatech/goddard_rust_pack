# Rust Lambda with AWS CDK

A production-ready Rust Lambda function deployed using AWS CDK and Cargo Lambda.

## 🏗️ Architecture

```
goddard-backend/
├── lambda/
│   └── hello-world/         # Rust Lambda function
│       ├── src/
│       │   └── main.rs      # Lambda handler
│       ├── Cargo.toml       # Rust dependencies
│       └── .cargo/
│           └── config.toml  # Build configuration
├── infrastructure/          # CDK infrastructure
│   ├── bin/
│   │   └── app.ts          # CDK app entry point
│   ├── lib/
│   │   └── rust-lambda-stack.ts  # Stack definition
│   ├── package.json        # Node dependencies
│   ├── tsconfig.json       # TypeScript config
│   └── cdk.json           # CDK configuration
├── scripts/                # Automation scripts
│   ├── build.sh           # Build script
│   ├── deploy.sh          # Deployment script
│   └── test-local.sh      # Local testing
├── docs/                  # Documentation
└── Makefile              # Task automation

```

## 🚀 Quick Start

### Prerequisites

1. **Rust & Cargo**: Install from [rustup.rs](https://rustup.rs/)
2. **Node.js**: Version 18+ recommended
3. **AWS CLI**: Configured with credentials
4. **AWS CDK**: Will be installed automatically
5. **Python 3**: For cargo-lambda installation

### Installation

```bash
# Install all dependencies
make install
```

### Local Development

```bash
# Run Lambda locally with hot reload
make test-local

# Test the endpoints:
curl http://localhost:9000/
curl http://localhost:9000/hello/World
curl http://localhost:9000/health
```

### Deployment

```bash
# First time setup (bootstrap CDK)
make bootstrap

# Deploy to AWS
make deploy
```

## 📝 Available Commands

```bash
make help         # Show all available commands
make install      # Install dependencies
make build        # Build Rust Lambda and CDK
make test         # Run tests
make test-local   # Run Lambda locally
make deploy       # Deploy to AWS
make synth        # Synthesize CDK stack
make destroy      # Destroy AWS resources
make clean        # Clean build artifacts
make diff         # Show CDK diff
```

## 🔧 API Endpoints

- `GET /` - Root endpoint returning welcome message
- `GET /hello/{name}` - Personalized greeting
- `GET /health` - Health check endpoint

## 🏃 Development Workflow

1. **Make changes** to Rust code in `lambda/hello-world/src/`
2. **Test locally** with `make test-local`
3. **Run tests** with `make test`
4. **Deploy** with `make deploy`

## 🔍 Monitoring

After deployment, you can:
- View logs in CloudWatch
- Monitor metrics in AWS Lambda console
- Access API Gateway dashboard for request metrics

## 🧪 Testing

```bash
# Run Rust unit tests
cd lambda/hello-world && cargo test

# Test deployed API
API_URL=$(aws cloudformation describe-stacks \
  --stack-name RustLambdaStack \
  --query 'Stacks[0].Outputs[?OutputKey==`ApiUrl`].OutputValue' \
  --output text)

curl $API_URL
curl $API_URL/hello/World
curl $API_URL/health
```

## 🛠️ Customization

### Adding New Endpoints

1. Edit `lambda/hello-world/src/main.rs` to add new handlers
2. Update the router in `create_app()` function
3. Add corresponding routes in `infrastructure/lib/rust-lambda-stack.ts`
4. Deploy with `make deploy`

### Environment Variables

Add environment variables in `infrastructure/lib/rust-lambda-stack.ts`:

```typescript
environment: {
  RUST_LOG: 'info',
  YOUR_VAR: 'value',
}
```

## 📊 Performance

- **Cold Start**: ~50-100ms (ARM64 + Rust)
- **Memory**: 256MB (configurable)
- **Timeout**: 30 seconds (configurable)
- **Architecture**: ARM64 for better price/performance

## 🔒 Security

- IAM roles with least privilege
- API Gateway with CORS configuration
- CloudWatch logging enabled
- No hardcoded secrets

## 🚨 Troubleshooting

### Build Issues
```bash
# Clean and rebuild
make clean
make build
```

### Deployment Issues
```bash
# Check AWS credentials
aws sts get-caller-identity

# Bootstrap CDK if needed
make bootstrap
```

### Local Testing Issues
```bash
# Ensure cargo-lambda is installed
pip3 install cargo-lambda
# Or
cargo install cargo-lambda
```

## 📚 Resources

- [Cargo Lambda Documentation](https://www.cargo-lambda.info/)
- [AWS CDK Documentation](https://docs.aws.amazon.com/cdk/)
- [Rust Lambda Runtime](https://github.com/awslabs/aws-lambda-rust-runtime)

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests
5. Submit a pull request

## 📄 License

MIT