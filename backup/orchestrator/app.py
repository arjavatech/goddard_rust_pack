"""Starts the isolated backup CodeBuild job from the IAM-protected API."""

import json
import os
from datetime import datetime, timezone

import boto3


codebuild = boto3.client("codebuild")


def handler(event, _context):
    project_name = os.environ["BACKUP_PROJECT_NAME"]
    request_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")

    response = codebuild.start_build(
        projectName=project_name,
        environmentVariablesOverride=[
            {
                "name": "BACKUP_REQUEST_ID",
                "value": request_id,
                "type": "PLAINTEXT",
            }
        ],
    )
    build = response["build"]
    body = {
        "backup_request_id": request_id,
        "build_id": build["id"],
        "build_arn": build["arn"],
        "status": build["buildStatus"],
    }

    return {
        "statusCode": 202,
        "headers": {"content-type": "application/json"},
        "body": json.dumps(body),
    }
