#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must come from Secrets Manager}"
: "${BACKUP_BUCKET:?BACKUP_BUCKET is required}"
: "${UPLOADS_BUCKET:?UPLOADS_BUCKET is required}"
: "${SUPABASE_PROJECT_REF:?SUPABASE_PROJECT_REF is required}"
: "${SUPABASE_CLI_VERSION:?SUPABASE_CLI_VERSION is required}"
: "${BACKUP_ENVIRONMENT:?BACKUP_ENVIRONMENT is required}"

backup_id="${BACKUP_REQUEST_ID:-$(date -u +%Y%m%dT%H%M%SZ)}-${CODEBUILD_BUILD_NUMBER}"
backup_prefix="${BACKUP_ENVIRONMENT}/${backup_id}"
work_dir="${CODEBUILD_SRC_DIR}/out"
export backup_id
mkdir -p "$work_dir"

# The Supabase CLI uses pg_dump with Supabase-aware filtering. CodeBuild is
# privileged so the CLI can start its short-lived Docker container.
npx --yes "supabase@${SUPABASE_CLI_VERSION}" db dump --db-url "$DATABASE_URL" \
  --role-only -f "$work_dir/roles.sql"
npx --yes "supabase@${SUPABASE_CLI_VERSION}" db dump --db-url "$DATABASE_URL" \
  -f "$work_dir/schema.sql"
npx --yes "supabase@${SUPABASE_CLI_VERSION}" db dump --db-url "$DATABASE_URL" \
  --data-only --use-copy -x "storage.buckets_vectors" -x "storage.vector_indexes" \
  -f "$work_dir/data.sql"

# Copy only the version that was current for each upload object. The manifest
# is also the source of truth used by restore.sh; historical source versions
# are not copied on every daily run.
python3 scripts/copy_current_uploads.py \
  --source-bucket "$UPLOADS_BUCKET" \
  --backup-bucket "$BACKUP_BUCKET" \
  --backup-prefix "$backup_prefix" \
  --manifest "$work_dir/uploads-manifest.json" \
  --restore-list "$work_dir/uploads-restore.tsv"

cp scripts/restore.sh "$work_dir/restore.sh"
cp supabase-recovery-config.inventory.yml "$work_dir/supabase-recovery-config.inventory.yml"
chmod 700 "$work_dir/restore.sh"

(
  cd "$work_dir"
  sha256sum roles.sql schema.sql data.sql uploads-manifest.json uploads-restore.tsv \
    restore.sh supabase-recovery-config.inventory.yml > checksums.sha256
)

python3 - "$work_dir/manifest.json" <<'PY'
import json
import os
import sys
from datetime import datetime, timezone

manifest = {
    "format_version": 1,
    "environment": os.environ["BACKUP_ENVIRONMENT"],
    "backup_id": os.environ["backup_id"],
    "supabase_project_ref": os.environ["SUPABASE_PROJECT_REF"],
    "created_at": datetime.now(timezone.utc).isoformat(),
    "supabase_cli_version": os.environ["SUPABASE_CLI_VERSION"],
    "artifacts": ["roles.sql", "schema.sql", "data.sql", "uploads-manifest.json", "uploads-restore.tsv", "checksums.sha256", "restore.sh", "supabase-recovery-config.inventory.yml"],
    "completion_marker": "manifest.json",
}
with open(sys.argv[1], "w", encoding="utf-8") as manifest_file:
    json.dump(manifest, manifest_file, indent=2, sort_keys=True)
    manifest_file.write("\n")
PY

# Upload the completion marker last. Consumers must consider a recovery point
# usable only after this object exists.
aws s3 cp "$work_dir/roles.sql" "s3://${BACKUP_BUCKET}/${backup_prefix}/roles.sql"
aws s3 cp "$work_dir/schema.sql" "s3://${BACKUP_BUCKET}/${backup_prefix}/schema.sql"
aws s3 cp "$work_dir/data.sql" "s3://${BACKUP_BUCKET}/${backup_prefix}/data.sql"
aws s3 cp "$work_dir/uploads-manifest.json" "s3://${BACKUP_BUCKET}/${backup_prefix}/uploads-manifest.json"
aws s3 cp "$work_dir/uploads-restore.tsv" "s3://${BACKUP_BUCKET}/${backup_prefix}/uploads-restore.tsv"
aws s3 cp "$work_dir/checksums.sha256" "s3://${BACKUP_BUCKET}/${backup_prefix}/checksums.sha256"
aws s3 cp "$work_dir/restore.sh" "s3://${BACKUP_BUCKET}/${backup_prefix}/restore.sh"
aws s3 cp "$work_dir/supabase-recovery-config.inventory.yml" "s3://${BACKUP_BUCKET}/${backup_prefix}/supabase-recovery-config.inventory.yml"
aws s3 cp "$work_dir/manifest.json" "s3://${BACKUP_BUCKET}/${backup_prefix}/manifest.json"

echo "Completed recovery point: s3://${BACKUP_BUCKET}/${backup_prefix}/manifest.json"
