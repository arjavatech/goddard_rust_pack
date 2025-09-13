# Lambda Architecture Configuration

## Overview

This project is configured to use **AWS Graviton2 (ARM64)** architecture for optimal price performance on AWS Lambda.

## Why ARM64/Graviton2?

Based on [AWS documentation](https://aws.amazon.com/blogs/compute/migrating-aws-lambda-functions-to-arm-based-aws-graviton2-processors/):

- **20% lower duration charges** compared to x86_64
- **Up to 34% better price performance**
- **Up to 19% better performance** for compute-intensive workloads
- **Improved encryption performance** and machine learning inference
- **Larger L2 cache per vCPU** reducing memory read time

## Configuration Details

### CDK Stack Configuration
```typescript
// infrastructure/lib/rust-lambda-stack.ts
const rustLambda = new lambda.Function(this, 'RustHelloWorldLambda', {
  runtime: lambda.Runtime.PROVIDED_AL2023, // Amazon Linux 2023 supports ARM64
  architecture: lambda.Architecture.ARM_64, // AWS Graviton2 processor
  // ... other configuration
});
```

### Build Configuration
The build scripts are configured to compile Rust code for ARM64:

```bash
# Uses cargo-lambda for optimized ARM64 builds
cargo lambda build --release --arm64
```

### Requirements

1. **cargo-lambda**: Required for ARM64 Lambda builds
   ```bash
   cargo install cargo-lambda
   ```

2. **Architecture Validation**: Build scripts automatically validate:
   - Binary is compiled for ARM64/aarch64
   - CDK configuration matches binary architecture

## Build Process

### Automated Build
```bash
# Build with architecture validation
make build

# Validate architecture configuration
make validate

# Or run scripts directly
./scripts/build.sh
./scripts/validate-architecture.sh
```

### Manual Build
```bash
cd lambda/hello-world
cargo lambda build --release --arm64
```

### Architecture Verification
The build script automatically verifies the binary architecture:
```bash
file target/lambda/hello-world/bootstrap
# Expected: ELF 64-bit LSB executable, aarch64, version 1 (SYSV)
```

## Deployment

When deploying, the CDK stack will:
1. Use the ARM64 binary from `target/lambda/hello-world/bootstrap`
2. Configure the Lambda function with `Architecture: ARM_64`
3. Ensure compatibility with `provided.al2023` runtime

## Performance Monitoring

Monitor performance improvements:
- **Duration**: Should see ~19% improvement for compute-intensive operations
- **Cost**: 20% lower duration charges
- **Memory**: Improved efficiency due to larger L2 cache

## Migration Notes

### From x86_64 to ARM64
If migrating from x86_64:

1. **Dependencies**: All Rust dependencies are compatible (no C bindings with architecture-specific code)
2. **Testing**: Existing tests continue to work without modification
3. **Deployment**: Single configuration change in CDK stack
4. **Rollback**: Can switch back by changing `Architecture.X86_64` in CDK

### Supported Runtimes
ARM64 is supported on all current Lambda runtimes:
- Node.js 18.x+
- Python 3.8+
- Java 8, 11, 17, 21
- .NET 6+
- Ruby 3.2+
- **Custom runtime (provided.al2023)** ← Used by this project

## Troubleshooting

### Build Issues
```bash
# Error: cargo-lambda not found
cargo install cargo-lambda

# Error: Wrong architecture
file target/lambda/hello-world/bootstrap
# Should show: aarch64 or ARM
```

### Deployment Issues
```bash
# Verify CDK configuration
grep -r "Architecture" infrastructure/lib/
# Should show: Architecture.ARM_64
```

### Performance Issues
- Check CloudWatch metrics for duration improvements
- Compare cold start times with previous x86_64 deployment
- Monitor cost savings in AWS billing

## References

- [AWS Lambda ARM64 Migration Guide](https://aws.amazon.com/blogs/compute/migrating-aws-lambda-functions-to-arm-based-aws-graviton2-processors/)
- [AWS Lambda Architecture Documentation](https://docs.aws.amazon.com/lambda/latest/dg/foundation-arch.html)
- [AWS Graviton Getting Started](https://github.com/aws/aws-graviton-getting-started)
- [cargo-lambda Documentation](https://www.cargo-lambda.info/)