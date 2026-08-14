# Supabase Backup API Runbook

Each environment has an independent IAM-authenticated API endpoint:

| Environment | Secret | Backup retention | S3 prefix |
| --- | --- | --- | --- |
| Dev | `goddard/dev/supabase-backup` | 90 days | `dev/` |
| Production | `goddard/prod/supabase-backup` | 365 days | `prod/` |

The endpoint starts a CodeBuild job. It does not mean the backup is complete;
the job succeeds only after it uploads the recovery point `manifest.json`.

## One-time setup

1. Create both Secrets Manager secrets. Each must contain JSON with a
   `database_url` key, using the matching Supabase Session Pooler or direct
   database connection string.
2. Deploy each stack with its project reference:

```bash
cd infrastructure
npx cdk deploy GoddardDevStack --parameters DevSupabaseProjectRef=DEV_PROJECT_REF
npx cdk deploy GoddardProdStack --parameters ProdSupabaseProjectRef=PROD_PROJECT_REF
```

From the repository root, the equivalent backup-only deployment commands are:

```bash
make backup-deploy-dev
make backup-deploy-prod
```

3. Record the `BackupApiPath` and `BackupApiInvokeArn` outputs from each stack.
   The Dev URL can start only Dev backups and the Production URL can start only
   Production backups.

## API access

The caller needs API Gateway IAM authorization, not an application API key.
Grant the external scheduler's dedicated AWS principal only this action against
the matching stack output ARN:

```json
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Action": "execute-api:Invoke",
    "Resource": "PASTE_THE_MATCHING_BackupApiInvokeArn_OUTPUT_HERE"
  }]
}
```

It must send an AWS SigV4-signed `POST` request with an empty JSON body to the
matching `BackupApiPath`. Do not place database credentials, bucket names, or
an environment selector in the request.

A `202` response contains a `build_id` and `backup_request_id`. Poll the
matching CodeBuild build ID until it succeeds, then require this object before
considering the recovery point usable:

```text
s3://<matching-backup-bucket>/<dev-or-prod>/<backup_request_id>-<build-number>/manifest.json
```

## External scheduler setup

Configure one job per environment, once every 24 hours. Give each job a
separate IAM principal and policy; never reuse a Dev scheduler credential for
Production. The scheduler must retry transport failures, treat non-`202`
responses as failed, and alert when the queued build fails or no manifest is
created that day.

The repository intentionally creates no EventBridge scheduler, rule, or DLQ.
Scheduling cadence and retry policy remain under the external scheduler's
control.

## Restore to a new Supabase project

Download `restore.sh` from the chosen completed S3 prefix and run it from a
secure operator machine with AWS CLI, `psql`, and `sha256sum`:

```bash
BACKUP_BUCKET='goddard-prod-backups-ACCOUNT-REGION' \
BACKUP_PREFIX='prod/20260814T000000Z-123' \
TARGET_DATABASE_URL='postgresql://...' \
TARGET_UPLOADS_BUCKET='goddard-uploads-recovery' \
CONFIRM_NEW_PROJECT_RESTORE=1 \
./restore.sh
```

Then apply the bundled `supabase-recovery-config.inventory.yml` and validate
Auth password login, data, RLS/triggers, and upload objects before routing
traffic to the restored project.
