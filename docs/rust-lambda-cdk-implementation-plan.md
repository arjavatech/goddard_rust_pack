# Rust Lambda + AWS CDK Implementation Plan

## Project Overview

This plan provides a complete implementation for:
- Rust Lambda function with "Hello World" API endpoint using Cargo Lambda
- AWS CDK TypeScript infrastructure for deployment
- Automated build and deployment pipeline
- Comprehensive testing strategy

## 📁 Recommended Directory Structure

```
rust-lambda-api/
├── README.md
├── Makefile                          # Build automation
├── .gitignore
├── lambda/                           # Rust Lambda function
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── src/
│   │   ├── main.rs                  # Lambda entry point
│   │   ├── lib.rs                   # Business logic
│   │   └── handlers/
│   │       └── hello.rs             # API handlers
│   ├── tests/
│   │   ├── integration_tests.rs
│   │   └── unit_tests.rs
│   └── target/                      # Build artifacts (gitignored)
├── infrastructure/                   # CDK TypeScript
│   ├── package.json
│   ├── package-lock.json
│   ├── tsconfig.json
│   ├── cdk.json
│   ├── jest.config.js
│   ├── src/
│   │   ├── main.ts                  # CDK app entry
│   │   ├── stacks/
│   │   │   └── rust-lambda-stack.ts # Lambda stack
│   │   └── constructs/
│   │       └── rust-lambda.ts       # Reusable construct
│   ├── test/
│   │   └── rust-lambda-stack.test.ts
│   └── node_modules/                # Dependencies (gitignored)
├── scripts/
│   ├── build.sh                     # Build all components
│   ├── deploy.sh                    # Deploy infrastructure
│   ├── test-local.sh               # Local testing
│   └── cleanup.sh                  # Resource cleanup
└── .github/
    └── workflows/
        └── ci-cd.yml               # GitHub Actions CI/CD
```

## 🛠️ Prerequisites and Dependencies

### System Requirements
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install Cargo Lambda
pip3 install cargo-lambda

# Install Node.js (v18+ recommended)
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs

# Install AWS CDK CLI
npm install -g aws-cdk

# Install AWS CLI v2
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
unzip awscliv2.zip
sudo ./aws/install

# Install Docker (for local testing)
sudo apt-get update
sudo apt-get install docker.io
sudo systemctl start docker
sudo usermod -aG docker $USER
```

### AWS Configuration
```bash
# Configure AWS credentials
aws configure
# Enter: Access Key ID, Secret Access Key, Region (e.g., us-east-1), Output format (json)

# Bootstrap CDK (one-time setup per region)
cdk bootstrap
```

## 🚀 Step-by-Step Implementation

### Phase 1: Project Initialization

```bash
# 1. Create project directory
mkdir rust-lambda-api
cd rust-lambda-api

# 2. Initialize Git repository
git init
echo "target/" > .gitignore
echo "node_modules/" >> .gitignore
echo ".env" >> .gitignore
echo "cdk.out/" >> .gitignore

# 3. Create directory structure
mkdir -p lambda/src/handlers
mkdir -p lambda/tests
mkdir -p infrastructure/src/{stacks,constructs}
mkdir -p infrastructure/test
mkdir -p scripts
mkdir -p .github/workflows
```

### Phase 2: Rust Lambda Setup

```bash
# Navigate to lambda directory
cd lambda

# Initialize Cargo project
cargo init --name hello-world-lambda

# Add Lambda dependencies to Cargo.toml
```

### Phase 3: CDK Infrastructure Setup

```bash
# Navigate to infrastructure directory
cd ../infrastructure

# Initialize TypeScript project
npm init -y
npm install aws-cdk-lib constructs
npm install -D @types/node typescript ts-node jest @types/jest ts-jest
npm install -D @aws-cdk/assert

# Initialize CDK project
cdk init --language typescript
```

## 📄 Configuration Files

### lambda/Cargo.toml
```toml
[package]
name = "hello-world-lambda"
version = "0.1.0"
edition = "2021"

[dependencies]
lambda_web = "0.2"
lambda_runtime = "0.8"
tokio = { version = "1.0", features = ["macros"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
tokio-test = "0.4"

[[bin]]
name = "bootstrap"
path = "src/main.rs"
```

### infrastructure/package.json
```json
{
  "name": "rust-lambda-infrastructure",
  "version": "1.0.0",
  "description": "CDK infrastructure for Rust Lambda API",
  "main": "lib/main.js",
  "scripts": {
    "build": "tsc",
    "watch": "tsc -w",
    "test": "jest",
    "cdk": "cdk",
    "deploy": "cdk deploy",
    "destroy": "cdk destroy"
  },
  "dependencies": {
    "aws-cdk-lib": "^2.100.0",
    "constructs": "^10.0.0"
  },
  "devDependencies": {
    "@types/jest": "^29.5.0",
    "@types/node": "^18.0.0",
    "jest": "^29.5.0",
    "ts-jest": "^29.1.0",
    "ts-node": "^10.9.0",
    "typescript": "^5.0.0"
  }
}
```

### infrastructure/cdk.json
```json
{
  "app": "npx ts-node src/main.ts",
  "watch": {
    "include": ["**"],
    "exclude": [
      "README.md",
      "cdk*.json",
      "**/*.d.ts",
      "**/*.js",
      "tsconfig.json",
      "package*.json",
      "yarn.lock",
      "node_modules",
      "test"
    ]
  },
  "context": {
    "@aws-cdk/aws-lambda:recognizeLayerVersion": true,
    "@aws-cdk/core:checkSecretUsage": true,
    "@aws-cdk/core:target": "aws-cdk-lib",
    "@aws-cdk-containers/ecs-service-extensions:enableDefaultLogDriver": true,
    "@aws-cdk/aws-ec2:uniqueImdsv2TemplateName": true,
    "@aws-cdk/aws-ecs:arnFormatIncludesClusterName": true,
    "@aws-cdk/core:validateSnapshotRemovalPolicy": true,
    "@aws-cdk/aws-codepipeline:crossAccountKeyAliasStackSafeResourceName": true,
    "@aws-cdk/aws-s3:createDefaultLoggingPolicy": true,
    "@aws-cdk/aws-sns-subscriptions:restrictSqsDescryption": true,
    "@aws-cdk/aws-apigateway:disableCloudWatchRole": true,
    "@aws-cdk/core:enablePartitionLiterals": true,
    "@aws-cdk/aws-events:eventsTargetQueueSameAccount": true,
    "@aws-cdk/aws-iam:minimizePolicies": true,
    "@aws-cdk/core:stackRelativeExports": true
  }
}
```

### infrastructure/tsconfig.json
```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["es2020"],
    "declaration": true,
    "strict": true,
    "noImplicitAny": true,
    "strictNullChecks": true,
    "noImplicitThis": true,
    "alwaysStrict": true,
    "noUnusedLocals": false,
    "noUnusedParameters": false,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": false,
    "inlineSourceMap": true,
    "inlineSources": true,
    "experimentalDecorators": true,
    "strictPropertyInitialization": false,
    "typeRoots": ["./node_modules/@types"]
  },
  "exclude": ["cdk.out"]
}
```

## 🧪 Testing Strategy

### Local Testing Setup
- **Unit Tests**: Rust unit tests with `cargo test`
- **Integration Tests**: CDK unit tests with Jest
- **Local API Testing**: Using `cargo lambda watch` for hot reload
- **End-to-End Testing**: Deploy to development environment

### Testing Commands
```bash
# Test Rust Lambda locally
cd lambda
cargo lambda watch

# Test CDK infrastructure
cd infrastructure
npm test

# Integration testing
make test-all
```

## 🔧 Build and Deployment Scripts

### Makefile
```makefile
.PHONY: build test deploy clean

build:
	@echo "Building Rust Lambda..."
	cd lambda && cargo lambda build --release
	@echo "Building CDK infrastructure..."
	cd infrastructure && npm run build

test:
	@echo "Testing Rust Lambda..."
	cd lambda && cargo test
	@echo "Testing CDK infrastructure..."
	cd infrastructure && npm test

deploy:
	@echo "Deploying infrastructure..."
	cd infrastructure && cdk deploy --require-approval never

clean:
	@echo "Cleaning build artifacts..."
	cd lambda && cargo clean
	cd infrastructure && rm -rf node_modules cdk.out

setup:
	@echo "Setting up project..."
	cd lambda && cargo build
	cd infrastructure && npm install

dev:
	@echo "Starting local development..."
	cd lambda && cargo lambda watch &
	cd infrastructure && npm run watch

destroy:
	@echo "Destroying infrastructure..."
	cd infrastructure && cdk destroy --force
```

## 📊 Implementation Timeline

1. **Setup (30 min)**: Install dependencies, create directory structure
2. **Rust Lambda (45 min)**: Implement Hello World API endpoint
3. **CDK Infrastructure (60 min)**: Create deployment stack
4. **Testing (30 min)**: Set up local and integration testing
5. **Scripts & Automation (30 min)**: Build and deployment scripts
6. **Documentation (15 min)**: README and deployment instructions

Total estimated time: **3.5 hours**

## 🎯 Success Criteria

- [ ] Rust Lambda responds to HTTP requests locally
- [ ] CDK successfully deploys Lambda to AWS
- [ ] API Gateway provides public endpoint
- [ ] All tests pass (unit and integration)
- [ ] Build scripts work without manual intervention
- [ ] Documentation is complete and actionable

## 🚨 Common Pitfalls to Avoid

1. **IAM Permissions**: Ensure CDK has proper deployment permissions
2. **Cargo Lambda Target**: Use correct target for AWS Lambda runtime
3. **API Gateway Configuration**: Proper CORS and integration setup
4. **Environment Variables**: Don't hardcode AWS regions or account IDs
5. **Build Optimization**: Use release builds for production deployment

This plan provides a complete, production-ready setup for Rust Lambda functions with CDK deployment infrastructure.