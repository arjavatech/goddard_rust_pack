#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   BACKUP_BUCKET=... BACKUP_PREFIX=dev/... TARGET_DATABASE_URL=... \
#   TARGET_UPLOADS_BUCKET=... CONFIRM_NEW_PROJECT_RESTORE=1 ./restore.sh
#
# Run this from an operator workstation or a secure CI runner. It restores to
# a newly-created Supabase project; it deliberately refuses in-place restores.

: "${BACKUP_BUCKET:?Set BACKUP_BUCKET}"
: "${BACKUP_PREFIX:?Set BACKUP_PREFIX, e.g. dev/20260814T000000Z-42}"
: "${TARGET_DATABASE_URL:?Set TARGET_DATABASE_URL for the new project}"
: "${TARGET_UPLOADS_BUCKET:?Set TARGET_UPLOADS_BUCKET for the new environment}"
: "${CONFIRM_NEW_PROJECT_RESTORE:?Set CONFIRM_NEW_PROJECT_RESTORE=1 after creating the target project}"

if [[ "$CONFIRM_NEW_PROJECT_RESTORE" != "1" ]]; then
  echo "Refusing restore: CONFIRM_NEW_PROJECT_RESTORE must equal 1." >&2
  exit 2
fi

work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT
for file in roles.sql schema.sql data.sql checksums.sha256 uploads-restore.tsv manifest.json supabase-recovery-config.inventory.yml; do
  aws s3 cp "s3://${BACKUP_BUCKET}/${BACKUP_PREFIX}/${file}" "$work_dir/${file}"
done

(
  cd "$work_dir"
  sha256sum --check checksums.sha256
)

psql \
  --single-transaction \
  --variable ON_ERROR_STOP=1 \
  --file "$work_dir/roles.sql" \
  --file "$work_dir/schema.sql" \
  --command 'SET session_replication_role = replica' \
  --file "$work_dir/data.sql" \
  --dbname "$TARGET_DATABASE_URL"

while IFS=$'\t' read -r backup_key source_key; do
  [[ -z "$backup_key" ]] && continue
  aws s3api copy-object \
    --bucket "$TARGET_UPLOADS_BUCKET" \
    --key "$source_key" \
    --copy-source "${BACKUP_BUCKET}/${backup_key}" >/dev/null
done < "$work_dir/uploads-restore.tsv"

echo "Database and upload objects restored."
echo "Before directing traffic to this project, apply every item in:"
echo "  $work_dir/supabase-recovery-config.inventory.yml"
