# Implementation Guide: AWS CDK + Rust Lambda

## Step-by-Step Implementation

### Step 1: Project Structure Setup

Create the recommended monorepo structure:

```bash
# From project root
mkdir -p {infrastructure/{bin,lib/{constructs,stacks,config}},lambda-functions/{shared/src,user-service/src,auth-service/src},scripts,tests/{integration,e2e}}
```

### Step 2: Rust Workspace Configuration

**lambda-functions/Cargo.toml** (Workspace manifest):
```toml
[workspace]
members = ["shared", "user-service", "auth-service"]
resolver = "2"

[workspace.dependencies]
# Lambda runtime dependencies
lambda_runtime = "0.8.1"
lambda_web = "0.2.1"
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# HTTP and API
reqwest = { version = "0.11", features = ["json"] }
uuid = { version = "1.0", features = ["v4", "serde"] }

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# AWS SDK
aws-sdk-dynamodb = "1.0"
aws-config = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

**lambda-functions/shared/Cargo.toml**:
```toml
[package]
name = "shared"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
```

### Step 3: CDK Project Setup

**infrastructure/package.json**:
```json
{
  "name": "goddard-infrastructure",
  "version": "0.1.0",
  "scripts": {
    "build": "tsc",
    "watch": "tsc -w",
    "test": "jest",
    "cdk": "cdk",
    "synth": "npm run build && cdk synth",
    "deploy": "npm run build && cdk deploy",
    "diff": "npm run build && cdk diff",
    "destroy": "npm run build && cdk destroy"
  },
  "dependencies": {
    "aws-cdk-lib": "^2.100.0",
    "constructs": "^10.3.0",
    "@types/node": "^20.0.0"
  },
  "devDependencies": {
    "@types/jest": "^29.0.0",
    "jest": "^29.0.0",
    "typescript": "^5.0.0",
    "ts-jest": "^29.0.0"
  }
}
```

### Step 4: Custom Rust Lambda Construct

**infrastructure/lib/constructs/rust-lambda.ts**:
```typescript
import * as path from 'path';
import { Duration, Size } from 'aws-cdk-lib';
import { Runtime, Function as LambdaFunction, Code, Architecture } from 'aws-cdk-lib/aws-lambda';
import { Role, ServicePrincipal, ManagedPolicy, PolicyDocument, PolicyStatement } from 'aws-cdk-lib/aws-iam';
import { Construct } from 'constructs';

export interface RustLambdaProps {
  functionName: string;
  lambdaPath: string;
  handler?: string;
  timeout?: Duration;
  memorySize?: number;
  environment?: { [key: string]: string };
  description?: string;
}

export class RustLambda extends Construct {
  public readonly function: LambdaFunction;
  public readonly role: Role;

  constructor(scope: Construct, id: string, props: RustLambdaProps) {
    super(scope, id);

    // Create IAM role for Lambda
    this.role = new Role(this, 'LambdaRole', {
      assumedBy: new ServicePrincipal('lambda.amazonaws.com'),
      managedPolicies: [
        ManagedPolicy.fromAwsManagedPolicyName('service-role/AWSLambdaBasicExecutionRole'),
      ],
      inlinePolicies: {
        LambdaPolicy: new PolicyDocument({
          statements: [
            new PolicyStatement({
              actions: [
                'secretsmanager:GetSecretValue',
                'dynamodb:GetItem',
                'dynamodb:PutItem',
                'dynamodb:UpdateItem',
                'dynamodb:DeleteItem',
                'dynamodb:Query',
                'dynamodb:Scan',
              ],
              resources: ['*'], // Restrict in production
            }),
          ],
        }),
      },
    });

    // Create Lambda function
    this.function = new LambdaFunction(this, 'Function', {
      functionName: props.functionName,
      runtime: Runtime.PROVIDED_AL2023,
      architecture: Architecture.ARM_64,
      handler: props.handler || 'bootstrap',
      code: Code.fromAsset(path.join(__dirname, '../../../target/lambda', props.lambdaPath)),
      role: this.role,
      timeout: props.timeout || Duration.seconds(30),
      memorySize: props.memorySize || 128,
      environment: props.environment,
      description: props.description,
      tracing: 'Active', // Enable X-Ray tracing
    });
  }

  public grantInvoke(principal: any) {
    this.function.grantInvoke(principal);
  }
}
```

### Step 5: Lambda Stack Implementation

**infrastructure/lib/stacks/lambda-stack.ts**:
```typescript
import { Stack, StackProps, RemovalPolicy } from 'aws-cdk-lib';
import { Table, AttributeType, BillingMode } from 'aws-cdk-lib/aws-dynamodb';
import { RestApi, LambdaIntegration, Cors } from 'aws-cdk-lib/aws-apigateway';
import { Secret } from 'aws-cdk-lib/aws-secretsmanager';
import { Construct } from 'constructs';
import { RustLambda } from '../constructs/rust-lambda';

export interface LambdaStackProps extends StackProps {
  environment: string;
}

export class LambdaStack extends Stack {
  public readonly api: RestApi;

  constructor(scope: Construct, id: string, props: LambdaStackProps) {
    super(scope, id, props);

    // DynamoDB Table
    const userTable = new Table(this, 'UserTable', {
      tableName: `goddard-users-${props.environment}`,
      partitionKey: { name: 'id', type: AttributeType.STRING },
      billingMode: BillingMode.PAY_PER_REQUEST,
      removalPolicy: props.environment === 'prod' ? RemovalPolicy.RETAIN : RemovalPolicy.DESTROY,
    });

    // Secrets
    const dbSecret = new Secret(this, 'DatabaseSecret', {
      secretName: `goddard-db-${props.environment}`,
      generateSecretString: {
        secretStringTemplate: JSON.stringify({ username: 'admin' }),
        generateStringKey: 'password',
      },
    });

    // Lambda Functions
    const userLambda = new RustLambda(this, 'UserLambda', {
      functionName: `goddard-user-service-${props.environment}`,
      lambdaPath: 'user-service',
      environment: {
        TABLE_NAME: userTable.tableName,
        SECRET_ARN: dbSecret.secretArn,
        RUST_LOG: props.environment === 'dev' ? 'debug' : 'info',
      },
      description: 'User service Lambda function',
    });

    const authLambda = new RustLambda(this, 'AuthLambda', {
      functionName: `goddard-auth-service-${props.environment}`,
      lambdaPath: 'auth-service',
      environment: {
        TABLE_NAME: userTable.tableName,
        SECRET_ARN: dbSecret.secretArn,
        JWT_SECRET: 'your-jwt-secret', // Use Secrets Manager in production
      },
      description: 'Authentication service Lambda function',
    });

    // Grant permissions
    userTable.grantReadWriteData(userLambda.function);
    userTable.grantReadWriteData(authLambda.function);
    dbSecret.grantRead(userLambda.function);
    dbSecret.grantRead(authLambda.function);

    // API Gateway
    this.api = new RestApi(this, 'Api', {
      restApiName: `goddard-api-${props.environment}`,
      description: 'Goddard Backend API',
      defaultCorsPreflightOptions: {
        allowOrigins: Cors.ALL_ORIGINS,
        allowMethods: Cors.ALL_METHODS,
        allowHeaders: ['Content-Type', 'Authorization'],
      },
    });

    // API Routes
    const users = this.api.root.addResource('users');
    users.addMethod('GET', new LambdaIntegration(userLambda.function));
    users.addMethod('POST', new LambdaIntegration(userLambda.function));
    
    const userById = users.addResource('{id}');
    userById.addMethod('GET', new LambdaIntegration(userLambda.function));
    userById.addMethod('PUT', new LambdaIntegration(userLambda.function));
    userById.addMethod('DELETE', new LambdaIntegration(userLambda.function));

    const auth = this.api.root.addResource('auth');
    auth.addResource('login').addMethod('POST', new LambdaIntegration(authLambda.function));
    auth.addResource('register').addMethod('POST', new LambdaIntegration(authLambda.function));
  }
}
```

### Step 6: Build Scripts

**scripts/build-lambdas.sh**:
```bash
#!/bin/bash
set -e

echo "🦀 Building Rust Lambda functions..."

# Ensure cargo-lambda is installed
if ! command -v cargo-lambda &> /dev/null; then
    echo "Installing cargo-lambda..."
    pip install cargo-lambda
fi

cd lambda-functions

# Clean previous builds
cargo clean

# Build all Lambda functions for ARM64
echo "Building user-service..."
cd user-service
cargo lambda build --release --arm64
cd ..

echo "Building auth-service..."
cd auth-service  
cargo lambda build --release --arm64
cd ..

# Create target directory structure for CDK
mkdir -p ../target/lambda
cp -r target/lambda/* ../target/lambda/

echo "✅ Rust Lambda build completed!"
```

**Makefile**:
```makefile
.PHONY: build-lambdas deploy-dev deploy-prod test clean

# Build all Lambda functions
build-lambdas:
	@echo "Building Rust Lambda functions..."
	@chmod +x scripts/build-lambdas.sh
	@./scripts/build-lambdas.sh

# Install dependencies
install:
	@echo "Installing dependencies..."
	@cd infrastructure && npm install
	@rustup target add aarch64-unknown-linux-gnu

# Deploy to development
deploy-dev: build-lambdas
	@echo "Deploying to development..."
	@cd infrastructure && npm run deploy -- --context environment=dev

# Deploy to production
deploy-prod: build-lambdas
	@echo "Deploying to production..."
	@cd infrastructure && npm run deploy -- --context environment=prod

# Run tests
test:
	@echo "Running tests..."
	@cd lambda-functions && cargo test
	@cd infrastructure && npm test

# Local testing
test-local:
	@echo "Starting local testing environment..."
	@cd lambda-functions && cargo lambda watch

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cd lambda-functions && cargo clean
	@cd infrastructure && npm run build
	@rm -rf target/

# Format code
fmt:
	@cd lambda-functions && cargo fmt
	@cd infrastructure && npm run format

# Lint code
lint:
	@cd lambda-functions && cargo clippy -- -D warnings
	@cd infrastructure && npm run lint
```

### Step 7: CI/CD Pipeline

**.github/workflows/ci.yml**:
```yaml
name: CI/CD Pipeline

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        target: aarch64-unknown-linux-gnu
        
    - name: Setup Node.js
      uses: actions/setup-node@v4
      with:
        node-version: '20'
        
    - name: Install cargo-lambda
      run: pip install cargo-lambda
      
    - name: Cache Rust dependencies
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          lambda-functions/target
        key: rust-${{ hashFiles('**/Cargo.lock') }}
        
    - name: Cache Node dependencies  
      uses: actions/cache@v3
      with:
        path: infrastructure/node_modules
        key: node-${{ hashFiles('infrastructure/package-lock.json') }}
        
    - name: Run Rust tests
      run: |
        cd lambda-functions
        cargo test
        
    - name: Run Rust linting
      run: |
        cd lambda-functions  
        cargo clippy -- -D warnings
        
    - name: Install CDK dependencies
      run: |
        cd infrastructure
        npm install
        
    - name: Run CDK tests
      run: |
        cd infrastructure
        npm test
        
    - name: Build Lambda functions
      run: make build-lambdas
      
    - name: CDK Synth
      run: |
        cd infrastructure
        npm run synth -- --context environment=dev

  deploy-dev:
    needs: test
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/develop'
    steps:
    - uses: actions/checkout@v4
    
    - name: Configure AWS credentials
      uses: aws-actions/configure-aws-credentials@v4
      with:
        aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
        aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
        aws-region: us-east-1
        
    - name: Deploy to Development
      run: make deploy-dev

  deploy-prod:
    needs: test
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    environment: production
    steps:
    - uses: actions/checkout@v4
    
    - name: Configure AWS credentials
      uses: aws-actions/configure-aws-credentials@v4
      with:
        aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID_PROD }}
        aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY_PROD }}
        aws-region: us-east-1
        
    - name: Deploy to Production
      run: make deploy-prod
```

## Development Workflow

### Local Development
```bash
# 1. Install dependencies
make install

# 2. Start local development
cargo lambda watch  # In terminal 1
npm run watch       # In terminal 2 (infrastructure/)

# 3. Test locally
cargo lambda invoke user-service --data-file test-data.json
```

### Testing Strategy
- **Unit Tests**: Rust `cargo test` + TypeScript `npm test`
- **Integration Tests**: Real AWS resources in dev environment
- **E2E Tests**: Full API testing with deployed infrastructure

### Environment Management
- **Development**: Auto-deploy on `develop` branch
- **Production**: Manual approval required on `main` branch
- **Configuration**: CDK context for environment-specific settings

This implementation provides a production-ready architecture for AWS CDK + Rust Lambda integration with proper separation of concerns, security best practices, and a streamlined development workflow.