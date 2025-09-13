# 🏫 Goddard School Database Setup

Complete database setup with comprehensive audit trails and production-ready features.

## 🚀 Quick Start

```bash
# 1. Setup environment
make env-setup

# 2. Edit .env file with your database credentials
nano .env

# 3. Setup database
make db-setup
```

## 📋 Available Commands

### Database Commands

| Command | Description |
|---------|-------------|
| `make db-setup` | 🚀 Complete database setup with full audit system |
| `make db-reset` | ⚠️ Reset database (DANGER: Drops all data) |
| `make db-status` | 📊 Check database status and table counts |
| `make db-backup` | 💾 Create database backup |
| `make db-console` | 🖥️ Open database console |
| `make db-test-audit` | 🧪 Test audit functionality |

### Environment Commands

| Command | Description |
|---------|-------------|
| `make env-setup` | ⚙️ Setup environment configuration |
| `make env-validate` | ✅ Validate environment configuration |
| `make quick-start` | 🚀 Quick start setup (env + db + install) |

## 🏗️ Database Architecture

### Core Tables (14 total)

1. **`schools`** - Multi-tenant root table
2. **`users`** - All system users (parents, teachers, admins)
3. **`children`** - Child information
4. **`classrooms`** - Classroom definitions with capacity management
5. **`enrollments`** - Central enrollment process hub
6. **`parent_additional_emails`** - Additional parent contact emails
7. **`form_templates`** - Form registry integrated with Fillout
8. **`class_form_overrides`** - Classroom-specific form requirements
9. **`student_form_assignments`** - Materialized form assignments
10. **`form_submissions`** - Form submission data from webhooks
11. **`documents`** - Document metadata for file uploads
12. **`enrollment_approval_audit`** - Complete approval audit trail
13. **`enrollment_communications`** - Communication tracking
14. **`waitlist`** - Waitlist management

### 🔍 Audit System Features

**All tables include:**
- ✅ `created_at` - Record creation timestamp
- ✅ `updated_at` - Last update timestamp
- ✅ `created_by` - User who created record
- ✅ `updated_by` - User who last updated record
- ✅ `is_active` - Soft delete flag (FALSE = deleted)

**Automatic Audit Triggers:**
- Auto-updates `updated_at` on every UPDATE
- Auto-sets `created_by`/`updated_by` from user context
- Respects `app.current_user_id` setting

### 🛡️ Security Features

- **Row Level Security (RLS)** enabled on all tables
- **Email validation** with regex constraints
- **Business rule enforcement** via triggers
- **Capacity management** prevents classroom overflow
- **Data integrity** with comprehensive foreign keys

### ⚡ Performance Features

- **40+ Optimized Indexes** with `WHERE is_active = TRUE`
- **Partial indexes** for frequently filtered queries
- **Composite indexes** for common query patterns
- **Audit field indexes** for change tracking

## 🔧 Environment Configuration

Required environment variables in `.env`:

```env
# Database Configuration
DATABASE_URL=postgresql://username:password@localhost:5432/goddard_db

# Supabase Configuration (if using Supabase)
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_ANON_KEY=your_anon_key_here
SUPABASE_SERVICE_ROLE_KEY=your_service_role_key_here
SUPABASE_PROJECT_ID=your_project_id_here
```

## 🧪 Testing Audit Functionality

```bash
# Test audit system
make db-test-audit

# Manual audit testing
make db-console
```

```sql
-- Set user context for audit tracking
SET app.current_user_id = 'your-user-uuid-here';

-- Create a record (audit fields auto-populated)
INSERT INTO schools (name, subdomain) VALUES ('Test School', 'test');

-- Update record (updated_at and updated_by auto-set)
UPDATE schools SET name = 'Updated School' WHERE subdomain = 'test';

-- View audit trail
SELECT * FROM get_audit_trail('schools', 'school-uuid-here');

-- Soft delete
SELECT soft_delete('schools', 'school-uuid-here', 'user-uuid-here');

-- Restore record
SELECT restore_record('schools', 'school-uuid-here', 'user-uuid-here');
```

## 📊 Database Monitoring

```bash
# Check database status
make db-status

# View table statistics
make db-console
\dt+

# Check indexes
\di
```

## 💾 Backup & Recovery

```bash
# Create backup
make db-backup

# Restore from backup
make db-restore BACKUP_FILE=goddard_backup_20241213_143022.sql
```

## 🚨 Troubleshooting

### Common Issues

**1. DATABASE_URL not set**
```bash
make env-setup
# Edit .env file with your database URL
```

**2. Connection refused**
```bash
# Check if PostgreSQL is running
pg_ctl status
# or
brew services list | grep postgresql
```

**3. Permission denied**
```bash
# Grant privileges to your user
GRANT ALL PRIVILEGES ON DATABASE goddard_db TO your_username;
```

**4. Reset everything**
```bash
make db-reset
# Type 'yes' to confirm
```

## 📈 Performance Tuning

### Query Optimization
- All queries should use `WHERE is_active = TRUE` for soft-deleted records
- Use indexes effectively with proper WHERE clauses
- Consider query patterns when adding new indexes

### Monitoring
```sql
-- Check slow queries
SELECT query, mean_time, calls
FROM pg_stat_statements
ORDER BY mean_time DESC
LIMIT 10;

-- Check index usage
SELECT schemaname, tablename, indexname, idx_scan
FROM pg_stat_user_indexes
ORDER BY idx_scan ASC;
```

## 🔐 Security Best Practices

1. **Always use parameterized queries** to prevent SQL injection
2. **Set user context** for audit tracking: `SET app.current_user_id = 'uuid'`
3. **Use soft delete** instead of hard delete: `SELECT soft_delete('table', 'id', 'user_id')`
4. **Validate email formats** are enforced at database level
5. **Row Level Security** policies should be configured per application needs

## 📝 Schema Evolution

### Adding New Tables
```sql
-- Follow the audit pattern
CREATE TABLE new_table (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- your columns here
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- Add audit triggers
CREATE TRIGGER update_new_table_audit
    BEFORE UPDATE ON new_table
    FOR EACH ROW EXECUTE FUNCTION update_audit_fields();

CREATE TRIGGER set_new_table_created_by
    BEFORE INSERT ON new_table
    FOR EACH ROW EXECUTE FUNCTION set_created_by();
```

### Migration Best Practices
1. Always backup before migrations
2. Test migrations on development first
3. Use transactions for atomic changes
4. Add indexes concurrently in production
5. Monitor performance after changes

---

## 🎉 Ready to Use!

Your Goddard School database is now production-ready with:
- ✅ Complete audit trails
- ✅ Soft delete system
- ✅ Performance optimization
- ✅ Security hardening
- ✅ Business logic enforcement
- ✅ Backup/restore capabilities

**Next Steps:**
1. Configure your application to use `app.current_user_id`
2. Implement RLS policies based on your access patterns
3. Set up monitoring and alerting
4. Configure automated backups