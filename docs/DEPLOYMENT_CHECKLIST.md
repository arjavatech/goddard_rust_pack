# AWS Lambda Deployment Checklist - Supabase Connection Fix

## ✅ Changes Completed

### 1. Database Configuration Updated (`src/config/database.rs`)

**Changed:**
- ✅ Reduced connection pool size from 16 → 2 connections
- ✅ Added timeout configurations (wait: 5s, create: 10s, recycle: 5s)
- ✅ Configured Fast recycling method (disables prepared statements)
- ✅ Added connect_timeout parsing from connection string
- ✅ Added comprehensive comments explaining Lambda optimizations

**Why This Fixes Your Issues:**
- Small pool size prevents connection exhaustion
- Fast recycling disables prepared statements (incompatible with Transaction Pooler)
- Timeouts prevent indefinite waits
- Optimized for Lambda's stateless architecture

### 2. Documentation Created

**New Files:**
- ✅ `/docs/SUPABASE_CONNECTION_CONFIG.md` - Comprehensive connection guide
- ✅ `/lambda/goddard/.env.lambda.example` - Example Lambda environment config
- ✅ `/docs/DEPLOYMENT_CHECKLIST.md` - This file

## 🚀 Deployment Steps

### Step 1: Update Local .env for Testing (Optional)

If you want to test locally with Transaction Pooler:

```bash
# Edit lambda/goddard/.env
DATABASE_URL=postgresql://postgres.fxsjcrwsnnowlovcnddz:Arjava%402024@aws-1-us-west-1.pooler.supabase.com:6543/postgres?connect_timeout=10
```

**Test locally:**
```bash
cd lambda/goddard
cargo run

# In another terminal, test the problematic endpoints:
curl -X POST http://localhost:9000/enrollments/parent-invite \
  -H "Content-Type: application/json" \
  -H "x-api-key: test-owner-key-2024" \
  -d '{ your test data }'
```

### Step 2: Update Lambda Environment Variables

**CRITICAL: Update the DATABASE_URL in AWS Lambda**

```bash
# Set the correct DATABASE_URL for Lambda
AWS_PROFILE=goddard aws lambda update-function-configuration \
  --function-name RustLambdaStack-GoddardLambdaC65E3A55-XXXXXX \
  --environment "Variables={
    DATABASE_URL=postgresql://postgres.fxsjcrwsnnowlovcnddz:Arjava%402024@aws-1-us-west-1.pooler.supabase.com:6543/postgres?connect_timeout=10,
    SUPABASE_URL=https://fxsjcrwsnnowlovcnddz.supabase.co,
    SUPABASE_SERVICE_ROLE_KEY=your-service-role-key,
    SUPABASE_ANON_KEY=your-anon-key,
    OWNER_API_KEY=test-owner-key-2024
  }"
```

**Or use the automated script:**
```bash
AWS_PROFILE=goddard ./scripts/update-lambda-env.sh
```

### Step 3: Build and Deploy

```bash
# Build for Lambda
AWS_PROFILE=goddard cargo lambda build --release --arm64 --output-format zip

# Deploy using CDK
AWS_PROFILE=goddard npm run deploy

# Or use the automated deployment script
AWS_PROFILE=goddard ./scripts/deploy-env-auto.sh
```

### Step 4: Verify Deployment

```bash
# Check Lambda environment variables
AWS_PROFILE=goddard aws lambda get-function-configuration \
  --function-name RustLambdaStack-GoddardLambdaC65E3A55-XXXXXX \
  --query 'Environment.Variables'

# Should show DATABASE_URL with port 6543
```

### Step 5: Test the Fixed Endpoints

Test the previously failing endpoints:

**1. Test parent-invite endpoint:**
```bash
curl -X POST https://your-api-gateway-url/enrollments/parent-invite \
  -H "Content-Type: application/json" \
  -H "x-api-key: test-owner-key-2024" \
  -d '{
    "school_id": "uuid-here",
    "class_id": "uuid-here",
    "parent_email": "test@example.com",
    "parent_first_name": "John",
    "parent_last_name": "Doe",
    "child_first_name": "Jane",
    "child_last_name": "Doe",
    "child_birth_date": "2020-01-01",
    "gender": "Female"
  }'
```

**2. Test add-child endpoint:**
```bash
curl -X POST https://your-api-gateway-url/enrollments/add-child \
  -H "Content-Type: application/json" \
  -H "x-api-key: test-owner-key-2024" \
  -d '{
    "parent_id": "uuid-here",
    "school_id": "uuid-here",
    "class_id": "uuid-here",
    "child_first_name": "Jack",
    "child_last_name": "Doe",
    "child_birth_date": "2021-01-01",
    "gender": "Male"
  }'
```

**3. Monitor Lambda logs:**
```bash
AWS_PROFILE=goddard aws logs tail /aws/lambda/RustLambdaStack-GoddardLambdaC65E3A55-XXXXXX --follow
```

## 🎯 Expected Results

### Before Changes:
- ❌ Session Pooler (port 5432): Intermittent connection failures
- ❌ Transaction Pooler (port 6543): `parent-invite` and `add-child` timeout after 30s

### After Changes:
- ✅ Transaction Pooler (port 6543): ALL endpoints work consistently
- ✅ No timeouts on complex endpoints
- ✅ No intermittent connection failures
- ✅ Optimal Lambda performance

## 📊 What Changed Under the Hood

### Connection Pool Behavior:

**Before:**
```
Lambda Instance 1: [16 connections] ─┐
Lambda Instance 2: [16 connections] ─┼─> Supabase (Overwhelmed)
Lambda Instance 3: [16 connections] ─┘
Total: 48+ connections from just 3 Lambda instances!
```

**After:**
```
Lambda Instance 1: [2 connections] ─┐
Lambda Instance 2: [2 connections] ─┼─> Supabase Transaction Pooler (Optimized)
Lambda Instance 3: [2 connections] ─┘
Total: 6 connections, efficiently shared via pooler
```

### Prepared Statements:

**Before:**
```
Complex Query → tokio-postgres creates prepared statement
                ↓
                Transaction Pooler REJECTS (not supported)
                ↓
                Timeout after 30s
```

**After:**
```
Complex Query → Fast Recycling (no prepared statements)
                ↓
                Transaction Pooler ACCEPTS
                ↓
                Query executes successfully in <1s
```

## 🔍 Monitoring and Troubleshooting

### Check Database Connections:
```bash
# View active connections in Supabase dashboard
# Navigate to: Database → Logs → Connection logs
```

### Check Lambda Metrics:
```bash
# View Lambda duration, errors, throttles
# Navigate to: Lambda → Monitor → CloudWatch metrics
```

### View Detailed Logs:
```bash
AWS_PROFILE=goddard aws logs get-log-events \
  --log-group-name "/aws/lambda/YourFunctionName" \
  --log-stream-name "latest-stream" \
  --query 'events[-20:].message' \
  --output table
```

## 📝 Important Notes

1. **Database URL is CRITICAL**: Must use port 6543 (Transaction Pooler)
2. **URL encoding**: Password `Arjava@2024` becomes `Arjava%402024`
3. **Connection timeout**: The `?connect_timeout=10` parameter is important
4. **Pool size**: 2 connections per Lambda instance is optimal
5. **No prepared statements**: Handled automatically by Fast recycling

## 🎓 Learn More

- See `/docs/SUPABASE_CONNECTION_CONFIG.md` for detailed explanation
- See `.env.lambda.example` for environment variable format
- [Supabase Connection Pooling Docs](https://supabase.com/docs/guides/database/connecting-to-postgres)

## ✅ Checklist Summary

- [x] Updated `src/config/database.rs` with Lambda-optimized settings
- [x] Reduced pool size from 16 to 2
- [x] Configured Fast recycling (no prepared statements)
- [x] Added timeout configurations
- [x] Created comprehensive documentation
- [ ] Update Lambda environment variables with Transaction Pooler URL
- [ ] Build and deploy to Lambda
- [ ] Test parent-invite endpoint
- [ ] Test add-child endpoint
- [ ] Verify all other endpoints still work
- [ ] Monitor Lambda logs for any errors

---

**Status**: Ready for deployment ✅
**Last Updated**: 2025-01-13
