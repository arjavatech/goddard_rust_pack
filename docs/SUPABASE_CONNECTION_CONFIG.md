# Supabase Database Connection Configuration for AWS Lambda

## Overview

This document explains the optimal database connection configuration for connecting to Supabase from AWS Lambda functions using Rust, tokio-postgres, and deadpool-postgres.

## Configuration Summary

### Connection Pooler Type: **Transaction Pooler (Port 6543)**

For AWS Lambda and other serverless environments, always use Supabase's **Transaction Pooler** on port 6543.

### Why Transaction Pooler?

1. **Designed for Serverless**: Optimized for short-lived, transient connections
2. **Connection Sharing**: Intelligently shares database connections only when queries are active
3. **High Concurrency**: Handles many more connections than Session Pooler or Direct connections
4. **No Connection Limits**: Works within Lambda's stateless architecture

## Required Environment Variables

### DATABASE_URL Format

```bash
# Transaction Pooler (Port 6543) - REQUIRED for Lambda
DATABASE_URL="postgresql://postgres.fxsjcrwsnnowlovcnddz:[YOUR-PASSWORD]@aws-1-us-west-1.pooler.supabase.com:6543/postgres?connect_timeout=10"
```

### Key Components:

1. **Host**: `aws-1-us-west-1.pooler.supabase.com` (your region may differ)
2. **Port**: `6543` (Transaction mode)
3. **Query Parameters**:
   - `connect_timeout=10` - 10 second connection timeout

## Connection Pool Configuration

Our implementation in `src/config/database.rs` uses these Lambda-optimized settings:

```rust
// Pool Size: 2 connections per Lambda instance
// Each Lambda instance only needs 1-2 connections
// Lambda's concurrency handles load distribution
PoolConfig::new(2)

// Timeouts:
- wait_timeout: 5 seconds   // Max wait for connection from pool
- create_timeout: 10 seconds // Max time to create new connection
- recycle_timeout: 5 seconds // Max time to recycle connection
```

### Why Small Pool Size?

- **Lambda Concurrency Model**: AWS scales by creating multiple Lambda instances, not by having many connections per instance
- **Prevents Connection Exhaustion**: Small pool size prevents hitting Supabase connection limits
- **Optimal Performance**: Each Lambda instance handles one request at a time, so 1-2 connections is sufficient

## Critical: Prepared Statements

### The Issue

Supabase Transaction Pooler (port 6543) **does NOT support prepared statements**. This is by design because:

1. Prepared statements are tied to a specific database session
2. Transaction pooler shares connections across multiple clients
3. Prepared statements would conflict with connection sharing

### The Solution

Our configuration uses `RecyclingMethod::Fast` to prevent prepared statement caching:

```rust
config.manager = Some(deadpool_postgres::ManagerConfig {
    recycling_method: deadpool_postgres::RecyclingMethod::Fast,
});
```

This ensures that:
- No prepared statements are cached between queries
- Each query is executed as a simple query
- Compatible with Supabase Transaction Pooler

## Comparison: Session vs Transaction Pooler

| Feature | Session Pooler (Port 5432) | Transaction Pooler (Port 6543) |
|---------|---------------------------|-------------------------------|
| **Best For** | Long-lived connections (VMs, persistent servers) | Serverless, Lambda, Edge Functions |
| **Connection Type** | Persistent, WebSocket-like | Transient, shared when active |
| **Prepared Statements** | ✅ Supported | ❌ Not Supported |
| **Max Connections** | Limited by pool size setting | Much higher capacity |
| **Lambda Suitability** | ❌ Poor (connection exhaustion) | ✅ Excellent |
| **Intermittent Failures** | ✅ Common with Lambda | ❌ Rare |

## Troubleshooting

### Issue: "Prepared statement already exists"

**Cause**: Using Session Pooler or prepared statements with Transaction Pooler

**Solution**:
1. Ensure you're using port 6543 (Transaction Pooler)
2. Verify `RecyclingMethod::Fast` is configured
3. Check DATABASE_URL uses correct port

### Issue: Connection Timeouts on Complex Queries

**Cause**: Previously caused by prepared statement conflicts with Transaction Pooler

**Solution**: Implemented in this configuration - uses Fast recycling method

### Issue: "Max client connections reached"

**Cause**: Too many connections per Lambda instance

**Solution**: Pool size reduced to 2 connections (already implemented)

## Best Practices

1. ✅ **Always use Transaction Pooler (port 6543) for Lambda**
2. ✅ **Keep pool size small (1-3 connections)**
3. ✅ **Add connect_timeout to connection string**
4. ✅ **Use Fast recycling method (no prepared statements)**
5. ✅ **Set appropriate timeouts (5-10 seconds)**
6. ❌ **Don't use Session Pooler for Lambda**
7. ❌ **Don't use large pool sizes (>5)**

## Testing

### Local Testing

Your local environment uses the same configuration, so you can test with:

```bash
# Ensure .env has Transaction Pooler URL
DATABASE_URL="postgresql://postgres.fxsjcrwsnnowlovcnddz:[YOUR-PASSWORD]@aws-1-us-west-1.pooler.supabase.com:6543/postgres?connect_timeout=10"

# Run locally
cargo run
```

### Lambda Testing

After deploying to Lambda, ensure environment variables are set:

```bash
# Check Lambda environment variables
AWS_PROFILE=goddard aws lambda get-function-configuration \
  --function-name YourFunctionName \
  --query 'Environment.Variables.DATABASE_URL'
```

## References

- [Supabase Connection Pooling Docs](https://supabase.com/docs/guides/database/connecting-to-postgres)
- [Supabase Transaction Mode](https://supabase.com/docs/guides/database/connecting-to-postgres#supavisor-transaction-mode)
- [AWS Lambda Best Practices](https://docs.aws.amazon.com/lambda/latest/dg/best-practices.html)

## Update History

- **2025-01-13**: Initial configuration optimized for AWS Lambda with Transaction Pooler
- Fixed timeout issues in `parent-invite` and `add-child` endpoints
- Configured Fast recycling to disable prepared statements
- Reduced pool size from 16 to 2 for Lambda optimization
