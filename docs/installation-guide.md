# Installation Guide - Rust Lambda + AWS CDK

## Prerequisites Installation

This guide provides step-by-step instructions to install all required dependencies for the Rust Lambda + AWS CDK project.

### 1. System Requirements

**Operating System Support:**
- Linux (Ubuntu 20.04+, Amazon Linux 2, etc.)
- macOS (10.15+)
- Windows 10/11 (with WSL2 recommended)

**Hardware Requirements:**
- Minimum 4GB RAM (8GB recommended)
- 10GB free disk space
- Internet connection for downloading dependencies

### 2. Core Tools Installation

#### Rust Programming Language

**Option 1: Official installer (Recommended)**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

**Option 2: Package managers**
```bash
# macOS with Homebrew
brew install rust

# Ubuntu/Debian
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows (PowerShell)
# Download from https://rustup.rs/ and run the installer
```

**Verify installation:**
```bash
rustc --version
cargo --version
```

#### Cargo Lambda

Cargo Lambda is essential for building and testing Lambda functions locally.

**Installation:**
```bash
# Using pip (recommended)
pip3 install cargo-lambda

# Using cargo
cargo install cargo-lambda

# macOS with Homebrew
brew install cargo-lambda
```

**Verify installation:**
```bash
cargo-lambda --version
```

#### Node.js and npm

**Option 1: Node Version Manager (Recommended)**
```bash
# Install nvm
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
source ~/.bashrc

# Install and use Node.js 18 LTS
nvm install 18
nvm use 18
nvm alias default 18
```

**Option 2: Official installer**
```bash
# macOS with Homebrew
brew install node@18

# Ubuntu/Debian
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs

# Windows - Download from https://nodejs.org/
```

**Verify installation:**
```bash
node --version  # Should be v18.x.x or higher
npm --version   # Should be 8.x.x or higher
```

#### AWS CDK CLI

```bash
npm install -g aws-cdk
```

**Verify installation:**
```bash
cdk --version
```

#### AWS CLI v2

**Linux/macOS:**
```bash
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
unzip awscliv2.zip
sudo ./aws/install
```

**macOS Alternative:**
```bash
brew install awscli
```

**Windows:**
Download and run the MSI installer from AWS documentation.

**Verify installation:**
```bash
aws --version  # Should be aws-cli/2.x.x
```

### 3. Development Tools (Optional but Recommended)

#### Code Editor Extensions

**VS Code Extensions:**
```bash
# Install VS Code first, then install extensions:
# - rust-analyzer
# - AWS Toolkit
# - TypeScript and JavaScript Support
```

#### Git (if not already installed)
```bash
# macOS
brew install git

# Ubuntu/Debian
sudo apt-get install git

# Windows
# Download from https://git-scm.com/download/win
```

#### Docker (for local testing)
```bash
# macOS
brew install docker

# Ubuntu/Debian
sudo apt-get update
sudo apt-get install docker.io
sudo systemctl start docker
sudo usermod -aG docker $USER

# Windows
# Download Docker Desktop from https://www.docker.com/products/docker-desktop/
```

### 4. AWS Configuration

#### Configure AWS Credentials

**Method 1: AWS CLI (Recommended)**
```bash
aws configure
```
Enter:
- Access Key ID
- Secret Access Key
- Default region (e.g., `us-east-1`)
- Default output format: `json`

**Method 2: Environment Variables**
```bash
export AWS_ACCESS_KEY_ID="your-access-key"
export AWS_SECRET_ACCESS_KEY="your-secret-key"
export AWS_DEFAULT_REGION="us-east-1"
```

**Method 3: AWS Credentials File**
Create `~/.aws/credentials`:
```ini
[default]
aws_access_key_id = your-access-key
aws_secret_access_key = your-secret-key

[default]
region = us-east-1
output = json
```

#### Bootstrap CDK (One-time setup per AWS account/region)
```bash
cdk bootstrap
```

### 5. Python Dependencies (for cargo-lambda)

Some systems may need Python development headers:

```bash
# Ubuntu/Debian
sudo apt-get install python3-dev python3-pip

# CentOS/RHEL
sudo yum install python3-devel python3-pip

# macOS (usually included)
# If issues occur, install via Homebrew:
brew install python@3.11
```

### 6. Additional Build Tools

#### Linux Build Essentials
```bash
# Ubuntu/Debian
sudo apt-get install build-essential pkg-config libssl-dev

# CentOS/RHEL
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel pkg-config
```

#### macOS Build Tools
```bash
xcode-select --install
```

### 7. Verification Script

Create a verification script to check all installations:

```bash
#!/bin/bash
echo "🔍 Checking Rust Lambda + AWS CDK Dependencies"
echo "================================================"

check_command() {
    if command -v "$1" >/dev/null 2>&1; then
        echo "✅ $1: $(command -v "$1")"
        if [ "$1" = "rustc" ]; then
            rustc --version
        elif [ "$1" = "node" ]; then
            node --version
        elif [ "$1" = "aws" ]; then
            aws --version
        elif [ "$1" = "cdk" ]; then
            cdk --version
        elif [ "$1" = "cargo-lambda" ]; then
            cargo-lambda --version
        fi
    else
        echo "❌ $1: Not found"
    fi
    echo
}

# Check all required tools
check_command "rustc"
check_command "cargo"
check_command "cargo-lambda"
check_command "node"
check_command "npm"
check_command "aws"
check_command "cdk"

# Check AWS credentials
echo "🔐 Checking AWS Configuration:"
if aws sts get-caller-identity >/dev/null 2>&1; then
    echo "✅ AWS credentials configured"
    aws sts get-caller-identity
else
    echo "❌ AWS credentials not configured or invalid"
fi
```

Save as `check-deps.sh` and run:
```bash
chmod +x check-deps.sh
./check-deps.sh
```

### 8. Platform-Specific Notes

#### Windows with WSL2
1. Install WSL2 and Ubuntu
2. Install all tools inside WSL2 environment
3. Configure VS Code to work with WSL2
4. Use Linux instructions within WSL2

#### macOS Apple Silicon (M1/M2)
```bash
# May need Rosetta 2 for some tools
softwareupdate --install-rosetta

# Use ARM64 versions when available
arch -arm64 brew install rust
```

#### Amazon Linux 2 / EC2
```bash
# Update system first
sudo yum update -y

# Install prerequisites
sudo yum install -y gcc gcc-c++ make git

# Follow standard Linux installation steps
```

### 9. Troubleshooting Common Issues

#### Rust/Cargo Issues
```bash
# Update Rust toolchain
rustup update

# Add necessary targets
rustup target add x86_64-unknown-linux-musl
```

#### Node.js Permission Issues
```bash
# Fix npm global permissions
mkdir ~/.npm-global
npm config set prefix '~/.npm-global'
# Add to ~/.bashrc: export PATH=~/.npm-global/bin:$PATH
```

#### AWS CLI Issues
```bash
# Check AWS CLI version (must be v2)
aws --version

# Reconfigure if needed
aws configure list
```

#### CDK Bootstrap Issues
```bash
# Bootstrap with specific region
cdk bootstrap aws://ACCOUNT-ID/REGION

# Or with profile
cdk bootstrap --profile your-profile-name
```

### 10. Quick Start Verification

After installation, test with a minimal project:

```bash
# Create test directory
mkdir rust-lambda-test && cd rust-lambda-test

# Initialize Rust project
cargo init --name test-lambda

# Test cargo-lambda
cargo lambda new test-function

# Initialize CDK project
mkdir cdk-test && cd cdk-test
cdk init app --language typescript

# If all commands succeed, installation is complete!
```

### 11. Development Environment Setup

#### Recommended Shell Configuration

Add to your `~/.bashrc` or `~/.zshrc`:
```bash
# Rust environment
source ~/.cargo/env

# Node.js global packages
export PATH=~/.npm-global/bin:$PATH

# AWS CLI completion
complete -C aws_completer aws

# Cargo Lambda completion (if available)
eval "$(cargo-lambda completions bash)"

# CDK aliases
alias cdkdiff="cdk diff"
alias cdkdeploy="cdk deploy"
alias cdksynth="cdk synth"
```

### 12. IDE/Editor Configuration

#### VS Code Settings
Create `.vscode/settings.json`:
```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "typescript.preferences.importModuleSpecifier": "relative",
  "aws.profile": "default",
  "aws.region": "us-east-1"
}
```

#### Recommended Extensions
- `rust-lang.rust-analyzer`
- `ms-vscode.vscode-typescript-next`
- `amazonwebservices.aws-toolkit-vscode`
- `bradlc.vscode-tailwindcss`

You're now ready to start building Rust Lambda functions with AWS CDK! 🚀