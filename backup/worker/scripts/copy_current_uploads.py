#!/usr/bin/env python3
"""Copy the live version of every upload and emit a restore manifest."""

import argparse
import csv
import json

import boto3


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-bucket", required=True)
    parser.add_argument("--backup-bucket", required=True)
    parser.add_argument("--backup-prefix", required=True)
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--restore-list", required=True)
    args = parser.parse_args()

    s3 = boto3.client("s3")
    objects = []
    paginator = s3.get_paginator("list_object_versions")
    for page in paginator.paginate(Bucket=args.source_bucket):
        for version in page.get("Versions", []):
            if not version["IsLatest"]:
                continue
            key = version["Key"]
            destination_key = f"{args.backup_prefix}/uploads/{key}"
            s3.copy_object(
                Bucket=args.backup_bucket,
                Key=destination_key,
                CopySource={
                    "Bucket": args.source_bucket,
                    "Key": key,
                    "VersionId": version["VersionId"],
                },
            )
            objects.append({
                "source_key": key,
                "source_version_id": version["VersionId"],
                "backup_key": destination_key,
                "etag": version["ETag"],
                "size": version["Size"],
                "last_modified": version["LastModified"].isoformat(),
            })

    with open(args.manifest, "w", encoding="utf-8") as output:
        json.dump({"source_bucket": args.source_bucket, "objects": objects}, output, indent=2)
        output.write("\n")
    with open(args.restore_list, "w", encoding="utf-8", newline="") as output:
        writer = csv.writer(output, delimiter="\t")
        for item in objects:
            writer.writerow([item["backup_key"], item["source_key"]])


if __name__ == "__main__":
    main()
