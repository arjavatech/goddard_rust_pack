"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.RustLambdaStack = void 0;
const cdk = require("aws-cdk-lib");
const lambda = require("aws-cdk-lib/aws-lambda");
const apigateway = require("aws-cdk-lib/aws-apigateway");
const logs = require("aws-cdk-lib/aws-logs");
const s3 = require("aws-cdk-lib/aws-s3");
const s3assets = require("aws-cdk-lib/aws-s3-assets");
const codebuild = require("aws-cdk-lib/aws-codebuild");
const cloudwatch = require("aws-cdk-lib/aws-cloudwatch");
const events = require("aws-cdk-lib/aws-events");
const targets = require("aws-cdk-lib/aws-events-targets");
const kms = require("aws-cdk-lib/aws-kms");
const iam = require("aws-cdk-lib/aws-iam");
const secretsmanager = require("aws-cdk-lib/aws-secretsmanager");
const path = require("path");
class RustLambdaStack extends cdk.Stack {
    constructor(scope, id, props) {
        super(scope, id, props);
        const { stage } = props;
        const stageName = stage.toUpperCase();
        // S3 bucket for product image uploads
        const uploadsBucket = new s3.Bucket(this, `Goddard${stageName}UploadsBucket`, {
            bucketName: `goddard-uploads-${stage}`,
            publicReadAccess: true,
            blockPublicAccess: s3.BlockPublicAccess.BLOCK_ACLS,
            cors: [
                {
                    allowedMethods: [s3.HttpMethods.GET, s3.HttpMethods.PUT],
                    allowedOrigins: ['*'],
                    allowedHeaders: ['*'],
                },
            ],
            removalPolicy: cdk.RemovalPolicy.RETAIN,
            versioned: true,
        });
        // Lambda function for Rust code
        // Using ARM64 architecture for up to 34% better price performance and 19% better performance
        // See: https://aws.amazon.com/blogs/compute/migrating-aws-lambda-functions-to-arm-based-aws-graviton2-processors/
        const rustLambda = new lambda.Function(this, `Goddard${stageName}Lambda`, {
            functionName: `goddard-${stage}`,
            runtime: lambda.Runtime.PROVIDED_AL2023, // Amazon Linux 2023 supports ARM64
            architecture: lambda.Architecture.ARM_64, // AWS Graviton2 processor (ARM64)
            handler: 'bootstrap',
            code: lambda.Code.fromAsset(path.join(__dirname, '../../lambda/goddard/target/lambda/goddard-backend'), {
                exclude: ['**', '!bootstrap'],
            }),
            memorySize: stage === 'dev' ? 128 : 256,
            timeout: cdk.Duration.seconds(30),
            environment: {
                RUST_LOG: 'info',
                S3_UPLOAD_BUCKET: uploadsBucket.bucketName,
                S3_BASE_URL: `https://${uploadsBucket.bucketRegionalDomainName}`,
            },
            logGroup: new logs.LogGroup(this, `Goddard${stageName}LambdaLogGroup`, {
                logGroupName: `/aws/lambda/goddard-${stage}`,
                retention: logs.RetentionDays.ONE_WEEK,
                removalPolicy: cdk.RemovalPolicy.DESTROY,
            }),
            description: `Goddard ${stageName} - Backend Lambda function with API endpoints`,
        });
        // Grant Lambda write access to the uploads bucket
        uploadsBucket.grantPut(rustLambda);
        // A separate, scheduled worker drains the durable FCM outbox. It does not
        // replace or expose the existing API Lambda, so mobile/API Gateway clients
        // retain their current endpoint and behavior.
        const notificationPushWorker = new lambda.Function(this, `Goddard${stageName}NotificationPushWorker`, {
            functionName: `goddard-${stage}-notification-push-worker`,
            runtime: lambda.Runtime.PROVIDED_AL2023,
            architecture: lambda.Architecture.ARM_64,
            handler: 'bootstrap',
            code: lambda.Code.fromAsset(path.join(__dirname, '../../lambda/goddard/target/lambda/notification_push_worker'), {
                exclude: ['**', '!bootstrap'],
            }),
            memorySize: 256,
            timeout: cdk.Duration.seconds(60),
            environment: { RUST_LOG: 'info' },
            logGroup: new logs.LogGroup(this, `Goddard${stageName}NotificationPushWorkerLogGroup`, {
                logGroupName: `/aws/lambda/goddard-${stage}-notification-push-worker`,
                retention: logs.RetentionDays.ONE_WEEK,
                removalPolicy: cdk.RemovalPolicy.DESTROY,
            }),
            description: `Goddard ${stageName} - reliable FCM push outbox worker`,
        });
        // Wake the worker after a committed outbox insert; the schedule below is
        // retained as the reliable retry/recovery path.
        notificationPushWorker.grantInvoke(rustLambda);
        new events.Rule(this, `Goddard${stageName}NotificationPushSchedule`, {
            description: `Drains Goddard ${stageName} FCM outbox once per minute.`,
            schedule: events.Schedule.rate(cdk.Duration.minutes(1)),
            targets: [new targets.LambdaFunction(notificationPushWorker)],
        });
        // API Gateway
        const api = new apigateway.RestApi(this, `Goddard${stageName}Api`, {
            restApiName: `Goddard ${stageName} API`,
            description: `${stageName} API Gateway for Goddard Backend Lambda function`,
            binaryMediaTypes: ['*/*'],
            deployOptions: {
                stageName: stage,
                tracingEnabled: stage === 'prod',
                metricsEnabled: true,
            },
            // CORS is handled entirely by Lambda middleware (cors.rs).
            // Do NOT use defaultCorsPreflightOptions here — it creates a MOCK
            // integration for OPTIONS that conflicts with binaryMediaTypes: ['*/*'],
            // causing API Gateway to corrupt/strip CORS headers from preflight responses.
        });
        // Lambda integration with proxy
        const lambdaIntegration = new apigateway.LambdaIntegration(rustLambda, {
            proxy: true,
        });
        // Handle root path
        api.root.addMethod('ANY', lambdaIntegration);
        // Explicit OPTIONS on root — ANY does NOT forward OPTIONS in REST API
        api.root.addMethod('OPTIONS', lambdaIntegration);
        // Create proxy resource for all other paths
        const proxyResource = api.root.addResource('{proxy+}');
        proxyResource.addMethod('ANY', lambdaIntegration);
        // Explicit OPTIONS on proxy — forwarded to Lambda CORS middleware
        proxyResource.addMethod('OPTIONS', lambdaIntegration);
        this.addBackupPipeline(api, uploadsBucket, stage);
        // Add CORS headers to API Gateway's own error responses (4XX/5XX)
        // so browsers can read error details instead of showing opaque CORS errors
        api.addGatewayResponse('Default4XX', {
            type: apigateway.ResponseType.DEFAULT_4XX,
            responseHeaders: {
                'method.response.header.Access-Control-Allow-Origin': "'*'",
                'method.response.header.Access-Control-Allow-Headers': "'Content-Type,Authorization,x-request-id,x-school-id,x-api-key'",
                'method.response.header.Access-Control-Allow-Methods': "'GET,POST,PUT,DELETE,OPTIONS,PATCH'",
            },
        });
        api.addGatewayResponse('Default5XX', {
            type: apigateway.ResponseType.DEFAULT_5XX,
            responseHeaders: {
                'method.response.header.Access-Control-Allow-Origin': "'*'",
                'method.response.header.Access-Control-Allow-Headers': "'Content-Type,Authorization,x-request-id,x-school-id,x-api-key'",
                'method.response.header.Access-Control-Allow-Methods': "'GET,POST,PUT,DELETE,OPTIONS,PATCH'",
            },
        });
        // Outputs
        new cdk.CfnOutput(this, 'ApiUrl', {
            value: api.url,
            description: `${stageName} API Gateway URL`,
            exportName: `Goddard${stageName}ApiUrl`,
        });
        new cdk.CfnOutput(this, 'LambdaFunctionName', {
            value: rustLambda.functionName,
            description: `${stageName} Lambda Function Name`,
            exportName: `Goddard${stageName}LambdaFunctionName`,
        });
        new cdk.CfnOutput(this, 'LambdaFunctionArn', {
            value: rustLambda.functionArn,
            description: `${stageName} Lambda Function ARN`,
            exportName: `Goddard${stageName}LambdaFunctionArn`,
        });
        new cdk.CfnOutput(this, 'NotificationPushWorkerFunctionName', {
            value: notificationPushWorker.functionName,
            description: `${stageName} FCM outbox worker function name`,
        });
        new cdk.CfnOutput(this, 'UploadsBucketName', {
            value: uploadsBucket.bucketName,
            description: `${stageName} S3 Uploads Bucket Name`,
            exportName: `Goddard${stageName}UploadsBucketName`,
        });
        new cdk.CfnOutput(this, 'UploadsBucketUrl', {
            value: `https://${uploadsBucket.bucketRegionalDomainName}`,
            description: `${stageName} S3 Uploads Bucket Base URL`,
            exportName: `Goddard${stageName}UploadsBucketUrl`,
        });
    }
    /**
     * The database backup is deliberately isolated from the API Lambda. The
     * Supabase CLI starts pg_dump in Docker, which is supported by privileged
     * CodeBuild but not by Lambda.
     */
    addBackupPipeline(api, uploadsBucket, stage) {
        const stageName = stage.toUpperCase();
        const stageId = stage === 'dev' ? 'Dev' : 'Prod';
        const retentionDays = stage === 'prod' ? 365 : 90;
        const backupKey = new kms.Key(this, `${stageId}BackupKey`, {
            alias: `alias/goddard-${stage}-backups`,
            enableKeyRotation: true,
            removalPolicy: cdk.RemovalPolicy.RETAIN,
        });
        const backupBucket = new s3.Bucket(this, `${stageId}BackupBucket`, {
            bucketName: cdk.Fn.sub(`goddard-${stage}-backups-\${AWS::AccountId}-\${AWS::Region}`),
            encryption: s3.BucketEncryption.KMS,
            encryptionKey: backupKey,
            bucketKeyEnabled: true,
            blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
            enforceSSL: true,
            versioned: true,
            removalPolicy: cdk.RemovalPolicy.RETAIN,
            lifecycleRules: [{
                    id: `expire-${stage}-recovery-points-after-${retentionDays}-days`,
                    enabled: true,
                    expiration: cdk.Duration.days(retentionDays),
                    noncurrentVersionExpiration: cdk.Duration.days(7),
                }],
        });
        // Create this secret before deploying and store a JSON value with the
        // `database_url` key. Keeping the value outside CloudFormation prevents
        // database credentials from appearing in templates or build logs.
        const databaseSecret = secretsmanager.Secret.fromSecretNameV2(this, `${stageId}SupabaseBackupDatabaseSecret`, `goddard/${stage}/supabase-backup`);
        const databaseSecretName = `goddard/${stage}/supabase-backup`;
        const projectRef = new cdk.CfnParameter(this, `${stageId}SupabaseProjectRef`, {
            type: 'String',
            description: `${stageName} Supabase project reference recorded in each backup manifest.`,
        });
        const workerSource = new s3assets.Asset(this, `${stageId}BackupWorkerSource`, {
            path: path.join(__dirname, '../../backup/worker'),
        });
        const backupProject = new codebuild.Project(this, `${stageId}SupabaseBackupProject`, {
            projectName: `goddard-${stage}-supabase-backup`,
            description: `Creates encrypted logical Supabase ${stageName} recovery bundles in S3.`,
            source: codebuild.Source.s3({
                bucket: workerSource.bucket,
                path: workerSource.s3ObjectKey,
            }),
            buildSpec: codebuild.BuildSpec.fromSourceFilename('buildspec.yml'),
            environment: {
                buildImage: codebuild.LinuxBuildImage.STANDARD_7_0,
                privileged: true,
                computeType: codebuild.ComputeType.MEDIUM,
                environmentVariables: {
                    DATABASE_URL: {
                        type: codebuild.BuildEnvironmentVariableType.SECRETS_MANAGER,
                        // Imported secrets have a partial ARN without Secrets Manager's
                        // random suffix. CodeBuild must resolve this by stable name.
                        value: `${databaseSecretName}:database_url`,
                    },
                    BACKUP_BUCKET: { value: backupBucket.bucketName },
                    UPLOADS_BUCKET: { value: uploadsBucket.bucketName },
                    BACKUP_ENVIRONMENT: { value: stage },
                    SUPABASE_PROJECT_REF: { value: projectRef.valueAsString },
                    SUPABASE_CLI_VERSION: { value: '2.67.1' },
                },
            },
            timeout: cdk.Duration.hours(2),
            queuedTimeout: cdk.Duration.minutes(30),
            concurrentBuildLimit: 1,
            encryptionKey: backupKey,
            logging: {
                cloudWatch: {
                    logGroup: new logs.LogGroup(this, `${stageId}SupabaseBackupBuildLogGroup`, {
                        retention: logs.RetentionDays.ONE_MONTH,
                        removalPolicy: cdk.RemovalPolicy.RETAIN,
                    }),
                },
            },
        });
        databaseSecret.grantRead(backupProject);
        workerSource.grantRead(backupProject);
        backupBucket.grantReadWrite(backupProject);
        uploadsBucket.grantRead(backupProject);
        const orchestrator = new lambda.Function(this, `${stageId}BackupOrchestrator`, {
            functionName: `goddard-${stage}-backup-orchestrator`,
            runtime: lambda.Runtime.PYTHON_3_12,
            architecture: lambda.Architecture.ARM_64,
            handler: 'app.handler',
            code: lambda.Code.fromAsset(path.join(__dirname, '../../backup/orchestrator')),
            timeout: cdk.Duration.seconds(30),
            memorySize: 256,
            environment: { BACKUP_PROJECT_NAME: backupProject.projectName },
            logGroup: new logs.LogGroup(this, `${stageId}BackupOrchestratorLogGroup`, {
                retention: logs.RetentionDays.ONE_MONTH,
                removalPolicy: cdk.RemovalPolicy.RETAIN,
            }),
        });
        orchestrator.addToRolePolicy(new iam.PolicyStatement({
            actions: ['codebuild:StartBuild'],
            resources: [backupProject.projectArn],
        }));
        const ops = api.root.addResource('ops');
        const backups = ops.addResource('backups');
        backups.addMethod('POST', new apigateway.LambdaIntegration(orchestrator), {
            authorizationType: apigateway.AuthorizationType.IAM,
        });
        new cloudwatch.Alarm(this, `${stageId}BackupBuildFailureAlarm`, {
            alarmDescription: `A ${stageName} Supabase backup CodeBuild job failed.`,
            metric: backupProject.metricFailedBuilds({ period: cdk.Duration.days(1) }),
            threshold: 1,
            evaluationPeriods: 1,
            treatMissingData: cloudwatch.TreatMissingData.NOT_BREACHING,
        });
        new cloudwatch.Alarm(this, `${stageId}BackupOrchestratorErrorAlarm`, {
            alarmDescription: `The ${stageName} Supabase backup orchestrator failed to start a build.`,
            metric: orchestrator.metricErrors({ period: cdk.Duration.days(1) }),
            threshold: 1,
            evaluationPeriods: 1,
            treatMissingData: cloudwatch.TreatMissingData.NOT_BREACHING,
        });
        new cdk.CfnOutput(this, 'BackupBucketName', { value: backupBucket.bucketName });
        new cdk.CfnOutput(this, 'BackupApiPath', {
            value: `${api.url}ops/backups`,
            description: `IAM-authenticated endpoint to manually start a ${stageName} backup.`,
        });
        new cdk.CfnOutput(this, 'BackupApiInvokeArn', {
            value: api.arnForExecuteApi('POST', '/ops/backups', '*'),
            description: `IAM resource ARN for invoking the ${stageName} backup endpoint.`,
        });
    }
}
exports.RustLambdaStack = RustLambdaStack;
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoicnVzdC1sYW1iZGEtc3RhY2suanMiLCJzb3VyY2VSb290IjoiIiwic291cmNlcyI6WyIuLi8uLi9saWIvcnVzdC1sYW1iZGEtc3RhY2sudHMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6Ijs7O0FBQUEsbUNBQW1DO0FBQ25DLGlEQUFpRDtBQUNqRCx5REFBeUQ7QUFDekQsNkNBQTZDO0FBQzdDLHlDQUF5QztBQUN6QyxzREFBc0Q7QUFDdEQsdURBQXVEO0FBQ3ZELHlEQUF5RDtBQUN6RCxpREFBaUQ7QUFDakQsMERBQTBEO0FBQzFELDJDQUEyQztBQUMzQywyQ0FBMkM7QUFDM0MsaUVBQWlFO0FBQ2pFLDZCQUE2QjtBQU83QixNQUFhLGVBQWdCLFNBQVEsR0FBRyxDQUFDLEtBQUs7SUFDNUMsWUFBWSxLQUFnQixFQUFFLEVBQVUsRUFBRSxLQUF1QjtRQUMvRCxLQUFLLENBQUMsS0FBSyxFQUFFLEVBQUUsRUFBRSxLQUFLLENBQUMsQ0FBQztRQUV4QixNQUFNLEVBQUUsS0FBSyxFQUFFLEdBQUcsS0FBSyxDQUFDO1FBQ3hCLE1BQU0sU0FBUyxHQUFHLEtBQUssQ0FBQyxXQUFXLEVBQUUsQ0FBQztRQUV0QyxzQ0FBc0M7UUFDdEMsTUFBTSxhQUFhLEdBQUcsSUFBSSxFQUFFLENBQUMsTUFBTSxDQUFDLElBQUksRUFBRSxVQUFVLFNBQVMsZUFBZSxFQUFFO1lBQzVFLFVBQVUsRUFBRSxtQkFBbUIsS0FBSyxFQUFFO1lBQ3RDLGdCQUFnQixFQUFFLElBQUk7WUFDdEIsaUJBQWlCLEVBQUUsRUFBRSxDQUFDLGlCQUFpQixDQUFDLFVBQVU7WUFDbEQsSUFBSSxFQUFFO2dCQUNKO29CQUNFLGNBQWMsRUFBRSxDQUFDLEVBQUUsQ0FBQyxXQUFXLENBQUMsR0FBRyxFQUFFLEVBQUUsQ0FBQyxXQUFXLENBQUMsR0FBRyxDQUFDO29CQUN4RCxjQUFjLEVBQUUsQ0FBQyxHQUFHLENBQUM7b0JBQ3JCLGNBQWMsRUFBRSxDQUFDLEdBQUcsQ0FBQztpQkFDdEI7YUFDRjtZQUNELGFB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QUUsa0JBQWtCLEVBQUUsRUFBRSxLQUFLLEVBQUUsWUFBWSxDQUFDLFVBQVUsRUFBRSxDQUFDLENBQUM7UUFDaEYsSUFBSSxHQUFHLENBQUMsU0FBUyxDQUFDLElBQUksRUFBRSxlQUFlLEVBQUU7WUFDdkMsS0FBSyxFQUFFLEdBQUcsR0FBRyxDQUFDLEdBQUcsYUFBYTtZQUM5QixXQUFXLEVBQUUsa0RBQWtELFNBQVMsVUFBVTtTQUNuRixDQUFDLENBQUM7UUFDSCxJQUFJLEdBQUcsQ0FBQyxTQUFTLENBQUMsSUFBSSxFQUFFLG9CQUFvQixFQUFFO1lBQzVDLEtBQUssRUFBRSxHQUFHLENBQUMsZ0JBQWdCLENBQUMsTUFBTSxFQUFFLGNBQWMsRUFBRSxHQUFHLENBQUM7WUFDeEQsV0FBVyxFQUFFLHFDQUFxQyxTQUFTLG1CQUFtQjtTQUMvRSxDQUFDLENBQUM7SUFDTCxDQUFDO0NBQ0Y7QUEvVEQsMENBK1RDIiwic291cmNlc0NvbnRlbnQiOlsiaW1wb3J0ICogYXMgY2RrIGZyb20gJ2F3cy1jZGstbGliJztcbmltcG9ydCAqIGFzIGxhbWJkYSBmcm9tICdhd3MtY2RrLWxpYi9hd3MtbGFtYmRhJztcbmltcG9ydCAqIGFzIGFwaWdhdGV3YXkgZnJvbSAnYXdzLWNkay1saWIvYXdzLWFwaWdhdGV3YXknO1xuaW1wb3J0ICogYXMgbG9ncyBmcm9tICdhd3MtY2RrLWxpYi9hd3MtbG9ncyc7XG5pbXBvcnQgKiBhcyBzMyBmcm9tICdhd3MtY2RrLWxpYi9hd3MtczMnO1xuaW1wb3J0ICogYXMgczNhc3NldHMgZnJvbSAnYXdzLWNkay1saWIvYXdzLXMzLWFzc2V0cyc7XG5pbXBvcnQgKiBhcyBjb2RlYnVpbGQgZnJvbSAnYXdzLWNkay1saWIvYXdzLWNvZGVidWlsZCc7XG5pbXBvcnQgKiBhcyBjbG91ZHdhdGNoIGZyb20gJ2F3cy1jZGstbGliL2F3cy1jbG91ZHdhdGNoJztcbmltcG9ydCAqIGFzIGV2ZW50cyBmcm9tICdhd3MtY2RrLWxpYi9hd3MtZXZlbnRzJztcbmltcG9ydCAqIGFzIHRhcmdldHMgZnJvbSAnYXdzLWNkay1saWIvYXdzLWV2ZW50cy10YXJnZXRzJztcbmltcG9ydCAqIGFzIGttcyBmcm9tICdhd3MtY2RrLWxpYi9hd3Mta21zJztcbmltcG9ydCAqIGFzIGlhbSBmcm9tICdhd3MtY2RrLWxpYi9hd3MtaWFtJztcbmltcG9ydCAqIGFzIHNlY3JldHNtYW5hZ2VyIGZyb20gJ2F3cy1jZGstbGliL2F3cy1zZWNyZXRzbWFuYWdlcic7XG5pbXBvcnQgKiBhcyBwYXRoIGZyb20gJ3BhdGgnO1xuaW1wb3J0IHsgQ29uc3RydWN0IH0gZnJvbSAnY29uc3RydWN0cyc7XG5cbmludGVyZmFjZSBHb2RkYXJTdGFja1Byb3BzIGV4dGVuZHMgY2RrLlN0YWNrUHJvcHMge1xuICBzdGFnZTogJ2RldicgfCAncHJvZCc7XG59XG5cbmV4cG9ydCBjbGFzcyBSdXN0TGFtYmRhU3RhY2sgZXh0ZW5kcyBjZGsuU3RhY2sge1xuICBjb25zdHJ1Y3RvcihzY29wZTogQ29uc3RydWN0LCBpZDogc3RyaW5nLCBwcm9wczogR29kZGFyU3RhY2tQcm9wcykge1xuICAgIHN1cGVyKHNjb3BlLCBpZCwgcHJvcHMpO1xuXG4gICAgY29uc3QgeyBzdGFnZSB9ID0gcHJvcHM7XG4gICAgY29uc3Qgc3RhZ2VOYW1lID0gc3RhZ2UudG9VcHBlckNhc2UoKTtcblxuICAgIC8vIFMzIGJ1Y2tldCBmb3IgcHJvZHVjdCBpbWFnZSB1cGxvYWRzXG4gICAgY29uc3QgdXBsb2Fkc0J1Y2tldCA9IG5ldyBzMy5CdWNrZXQodGhpcywgYEdvZGRhcmQke3N0YWdlTmFtZX1VcGxvYWRzQnVja2V0YCwge1xuICAgICAgYnVja2V0TmFtZTogYGdvZGRhcmQtdXBsb2Fkcy0ke3N0YWdlfWAsXG4gICAgICBwdWJsaWNSZWFkQWNjZXNzOiB0cnVlLFxuICAgICAgYmxvY2tQdWJsaWNBY2Nlc3M6IHMzLkJsb2NrUHVibGljQWNjZXNzLkJMT0NLX0FDTFMsXG4gICAgICBjb3JzOiBbXG4gICAgICAgIHtcbiAgICAgICAgICBhbGxvd2VkTWV0aG9kczogW3MzLkh0dHBNZXRob2RzLkdFVCwgczMuSHR0cE1ldGhvZHMuUFVUXSxcbiAgICAgICAgICBhbGxvd2VkT3JpZ2luczogWycqJ10sXG4gICAgICAgICAgYWxsb3dlZEhlYWRlcnM6IFsnKiddLFxuICAgICAgICB9LFxuICAgICAgXSxcbiAgICAgIHJlbW92YWxQb2xpY3k6IGNkay5SZW1vdmFsUG9saWN5LlJFVEFJTixcbiAgICAgIHZlcnNpb25lZDogdHJ1ZSxcbiAgICB9KTtcblxuICAgIC8vIExhbWJkYSBmdW5jdGlvbiBmb3IgUnVzdCBjb2RlXG4gICAgLy8gVXNpbmcgQVJNNjQgYXJjaGl0ZWN0dXJlIGZvciB1cCB0byAzNCUgYmV0dGVyIHByaWNlIHBlcmZvcm1hbmNlIGFuZCAxOSUgYmV0dGVyIHBlcmZvcm1hbmNlXG4gICAgLy8gU2VlOiBodHRwczovL2F3cy5hbWF6b24uY29tL2Jsb2dzL2NvbXB1dGUvbWlncmF0aW5nLWF3cy1sYW1iZGEtZnVuY3Rpb25zLXRvLWFybS1iYXNlZC1hd3MtZ3Jhdml0b24yLXByb2Nlc3NvcnMvXG4gICAgY29uc3QgcnVzdExhbWJkYSA9IG5ldyBsYW1iZGEuRnVuY3Rpb24odGhpcywgYEdvZGRhcmQke3N0YWdlTmFtZX1MYW1iZGFgLCB7XG4gICAgICBmdW5jdGlvbk5hbWU6IGBnb2RkYXJkLSR7c3RhZ2V9YCxcbiAgICAgIHJ1bnRpbWU6IGxhbWJkYS5SdW50aW1lLlBST1ZJREVEX0FMMjAyMywgLy8gQW1hem9uIExpbnV4IDIwMjMgc3VwcG9ydHMgQVJNNjRcbiAgICAgIGFyY2hpdGVjdHVyZTogbGFtYmRhLkFyY2hpdGVjdHVyZS5BUk1fNjQsIC8vIEFXUyBHcmF2aXRvbjIgcHJvY2Vzc29yIChBUk02NClcbiAgICAgIGhhbmRsZXI6ICdib290c3RyYXAnLFxuICAgICAgY29kZTogbGFtYmRhLkNvZGUuZnJvbUFzc2V0KHBhdGguam9pbihfX2Rpcm5hbWUsICcuLi8uLi9sYW1iZGEvZ29kZGFyZC90YXJnZXQvbGFtYmRhL2dvZGRhcmQtYmFja2VuZCcpLCB7XG4gICAgICAgIGV4Y2x1ZGU6IFsnKionLCAnIWJvb3RzdHJhcCddLFxuICAgICAgfSksXG4gICAgICBtZW1vcnlTaXplOiBzdGFnZSA9PT0gJ2RldicgPyAxMjggOiAyNTYsXG4gICAgICB0aW1lb3V0OiBjZGsuRHVyYXRpb24uc2Vjb25kcygzMCksXG4gICAgICBlbnZpcm9ubWVudDoge1xuICAgICAgICBSVVNUX0xPRzogJ2luZm8nLFxuICAgICAgICBTM19VUExPQURfQlVDS0VUOiB1cGxvYWRzQnVja2V0LmJ1Y2tldE5hbWUsXG4gICAgICAgIFMzX0JBU0VfVVJMOiBgaHR0cHM6Ly8ke3VwbG9hZHNCdWNrZXQuYnVja2V0UmVnaW9uYWxEb21haW5OYW1lfWAsXG4gICAgICB9LFxuICAgICAgbG9nR3JvdXA6IG5ldyBsb2dzLkxvZ0dyb3VwKHRoaXMsIGBHb2RkYXJkJHtzdGFnZU5hbWV9TGFtYmRhTG9nR3JvdXBgLCB7XG4gICAgICAgIGxvZ0dyb3VwTmFtZTogYC9hd3MvbGFtYmRhL2dvZGRhcmQtJHtzdGFnZX1gLFxuICAgICAgICByZXRlbnRpb246IGxvZ3MuUmV0ZW50aW9uRGF5cy5PTkVfV0VFSyxcbiAgICAgICAgcmVtb3ZhbFBvbGljeTogY2RrLlJlbW92YWxQb2xpY3kuREVTVFJPWSxcbiAgICAgIH0pLFxuICAgICAgZGVzY3JpcHRpb246IGBHb2RkYXJkICR7c3RhZ2VOYW1lfSAtIEJhY2tlbmQgTGFtYmRhIGZ1bmN0aW9uIHdpdGggQVBJIGVuZHBvaW50c2AsXG4gICAgfSk7XG5cbiAgICAvLyBHcmFudCBMYW1iZGEgd3JpdGUgYWNjZXNzIHRvIHRoZSB1cGxvYWRzIGJ1Y2tldFxuICAgIHVwbG9hZHNCdWNrZXQuZ3JhbnRQdXQocnVzdExhbWJkYSk7XG5cbiAgICAvLyBBIHNlcGFyYXRlLCBzY2hlZHVsZWQgd29ya2VyIGRyYWlucyB0aGUgZHVyYWJsZSBGQ00gb3V0Ym94LiBJdCBkb2VzIG5vdFxuICAgIC8vIHJlcGxhY2Ugb3IgZXhwb3NlIHRoZSBleGlzdGluZyBBUEkgTGFtYmRhLCBzbyBtb2JpbGUvQVBJIEdhdGV3YXkgY2xpZW50c1xuICAgIC8vIHJldGFpbiB0aGVpciBjdXJyZW50IGVuZHBvaW50IGFuZCBiZWhhdmlvci5cbiAgICBjb25zdCBub3RpZmljYXRpb25QdXNoV29ya2VyID0gbmV3IGxhbWJkYS5GdW5jdGlvbih0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfU5vdGlmaWNhdGlvblB1c2hXb3JrZXJgLCB7XG4gICAgICBmdW5jdGlvbk5hbWU6IGBnb2RkYXJkLSR7c3RhZ2V9LW5vdGlmaWNhdGlvbi1wdXNoLXdvcmtlcmAsXG4gICAgICBydW50aW1lOiBsYW1iZGEuUnVudGltZS5QUk9WSURFRF9BTDIwMjMsXG4gICAgICBhcmNoaXRlY3R1cmU6IGxhbWJkYS5BcmNoaXRlY3R1cmUuQVJNXzY0LFxuICAgICAgaGFuZGxlcjogJ2Jvb3RzdHJhcCcsXG4gICAgICBjb2RlOiBsYW1iZGEuQ29kZS5mcm9tQXNzZXQocGF0aC5qb2luKF9fZGlybmFtZSwgJy4uLy4uL2xhbWJkYS9nb2RkYXJkL3RhcmdldC9sYW1iZGEvbm90aWZpY2F0aW9uX3B1c2hfd29ya2VyJyksIHtcbiAgICAgICAgZXhjbHVkZTogWycqKicsICchYm9vdHN0cmFwJ10sXG4gICAgICB9KSxcbiAgICAgIG1lbW9yeVNpemU6IDI1NixcbiAgICAgIHRpbWVvdXQ6IGNkay5EdXJhdGlvbi5zZWNvbmRzKDYwKSxcbiAgICAgIGVudmlyb25tZW50OiB7IFJVU1RfTE9HOiAnaW5mbycgfSxcbiAgICAgIGxvZ0dyb3VwOiBuZXcgbG9ncy5Mb2dHcm91cCh0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfU5vdGlmaWNhdGlvblB1c2hXb3JrZXJMb2dHcm91cGAsIHtcbiAgICAgICAgbG9nR3JvdXBOYW1lOiBgL2F3cy9sYW1iZGEvZ29kZGFyZC0ke3N0YWdlfS1ub3RpZmljYXRpb24tcHVzaC13b3JrZXJgLFxuICAgICAgICByZXRlbnRpb246IGxvZ3MuUmV0ZW50aW9uRGF5cy5PTkVfV0VFSyxcbiAgICAgICAgcmVtb3ZhbFBvbGljeTogY2RrLlJlbW92YWxQb2xpY3kuREVTVFJPWSxcbiAgICAgIH0pLFxuICAgICAgZGVzY3JpcHRpb246IGBHb2RkYXJkICR7c3RhZ2VOYW1lfSAtIHJlbGlhYmxlIEZDTSBwdXNoIG91dGJveCB3b3JrZXJgLFxuICAgIH0pO1xuICAgIC8vIFdha2UgdGhlIHdvcmtlciBhZnRlciBhIGNvbW1pdHRlZCBvdXRib3ggaW5zZXJ0OyB0aGUgc2NoZWR1bGUgYmVsb3cgaXNcbiAgICAvLyByZXRhaW5lZCBhcyB0aGUgcmVsaWFibGUgcmV0cnkvcmVjb3ZlcnkgcGF0aC5cbiAgICBub3RpZmljYXRpb25QdXNoV29ya2VyLmdyYW50SW52b2tlKHJ1c3RMYW1iZGEpO1xuICAgIG5ldyBldmVudHMuUnVsZSh0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfU5vdGlmaWNhdGlvblB1c2hTY2hlZHVsZWAsIHtcbiAgICAgIGRlc2NyaXB0aW9uOiBgRHJhaW5zIEdvZGRhcmQgJHtzdGFnZU5hbWV9IEZDTSBvdXRib3ggb25jZSBwZXIgbWludXRlLmAsXG4gICAgICBzY2hlZHVsZTogZXZlbnRzLlNjaGVkdWxlLnJhdGUoY2RrLkR1cmF0aW9uLm1pbnV0ZXMoMSkpLFxuICAgICAgdGFyZ2V0czogW25ldyB0YXJnZXRzLkxhbWJkYUZ1bmN0aW9uKG5vdGlmaWNhdGlvblB1c2hXb3JrZXIpXSxcbiAgICB9KTtcblxuICAgIC8vIEFQSSBHYXRld2F5XG4gICAgY29uc3QgYXBpID0gbmV3IGFwaWdhdGV3YXkuUmVzdEFwaSh0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfUFwaWAsIHtcbiAgICAgIHJlc3RBcGlOYW1lOiBgR29kZGFyZCAke3N0YWdlTmFtZX0gQVBJYCxcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IEFQSSBHYXRld2F5IGZvciBHb2RkYXJkIEJhY2tlbmQgTGFtYmRhIGZ1bmN0aW9uYCxcbiAgICAgIGJpbmFyeU1lZGlhVHlwZXM6IFsnKi8qJ10sXG4gICAgICBkZXBsb3lPcHRpb25zOiB7XG4gICAgICAgIHN0YWdlTmFtZTogc3RhZ2UsXG4gICAgICAgIHRyYWNpbmdFbmFibGVkOiBzdGFnZSA9PT0gJ3Byb2QnLFxuICAgICAgICBtZXRyaWNzRW5hYmxlZDogdHJ1ZSxcbiAgICAgIH0sXG4gICAgICAvLyBDT1JTIGlzIGhhbmRsZWQgZW50aXJlbHkgYnkgTGFtYmRhIG1pZGRsZXdhcmUgKGNvcnMucnMpLlxuICAgICAgLy8gRG8gTk9UIHVzZSBkZWZhdWx0Q29yc1ByZWZsaWdodE9wdGlvbnMgaGVyZSDigJQgaXQgY3JlYXRlcyBhIE1PQ0tcbiAgICAgIC8vIGludGVncmF0aW9uIGZvciBPUFRJT05TIHRoYXQgY29uZmxpY3RzIHdpdGggYmluYXJ5TWVkaWFUeXBlczogWycqLyonXSxcbiAgICAgIC8vIGNhdXNpbmcgQVBJIEdhdGV3YXkgdG8gY29ycnVwdC9zdHJpcCBDT1JTIGhlYWRlcnMgZnJvbSBwcmVmbGlnaHQgcmVzcG9uc2VzLlxuICAgIH0pO1xuXG4gICAgLy8gTGFtYmRhIGludGVncmF0aW9uIHdpdGggcHJveHlcbiAgICBjb25zdCBsYW1iZGFJbnRlZ3JhdGlvbiA9IG5ldyBhcGlnYXRld2F5LkxhbWJkYUludGVncmF0aW9uKHJ1c3RMYW1iZGEsIHtcbiAgICAgIHByb3h5OiB0cnVlLFxuICAgIH0pO1xuXG4gICAgLy8gSGFuZGxlIHJvb3QgcGF0aFxuICAgIGFwaS5yb290LmFkZE1ldGhvZCgnQU5ZJywgbGFtYmRhSW50ZWdyYXRpb24pO1xuICAgIC8vIEV4cGxpY2l0IE9QVElPTlMgb24gcm9vdCDigJQgQU5ZIGRvZXMgTk9UIGZvcndhcmQgT1BUSU9OUyBpbiBSRVNUIEFQSVxuICAgIGFwaS5yb290LmFkZE1ldGhvZCgnT1BUSU9OUycsIGxhbWJkYUludGVncmF0aW9uKTtcblxuICAgIC8vIENyZWF0ZSBwcm94eSByZXNvdXJjZSBmb3IgYWxsIG90aGVyIHBhdGhzXG4gICAgY29uc3QgcHJveHlSZXNvdXJjZSA9IGFwaS5yb290LmFkZFJlc291cmNlKCd7cHJveHkrfScpO1xuICAgIHByb3h5UmVzb3VyY2UuYWRkTWV0aG9kKCdBTlknLCBsYW1iZGFJbnRlZ3JhdGlvbik7XG4gICAgLy8gRXhwbGljaXQgT1BUSU9OUyBvbiBwcm94eSDigJQgZm9yd2FyZGVkIHRvIExhbWJkYSBDT1JTIG1pZGRsZXdhcmVcbiAgICBwcm94eVJlc291cmNlLmFkZE1ldGhvZCgnT1BUSU9OUycsIGxhbWJkYUludGVncmF0aW9uKTtcblxuICAgIHRoaXMuYWRkQmFja3VwUGlwZWxpbmUoYXBpLCB1cGxvYWRzQnVja2V0LCBzdGFnZSk7XG5cbiAgICAvLyBBZGQgQ09SUyBoZWFkZXJzIHRvIEFQSSBHYXRld2F5J3Mgb3duIGVycm9yIHJlc3BvbnNlcyAoNFhYLzVYWClcbiAgICAvLyBzbyBicm93c2VycyBjYW4gcmVhZCBlcnJvciBkZXRhaWxzIGluc3RlYWQgb2Ygc2hvd2luZyBvcGFxdWUgQ09SUyBlcnJvcnNcbiAgICBhcGkuYWRkR2F0ZXdheVJlc3BvbnNlKCdEZWZhdWx0NFhYJywge1xuICAgICAgdHlwZTogYXBpZ2F0ZXdheS5SZXNwb25zZVR5cGUuREVGQVVMVF80WFgsXG4gICAgICByZXNwb25zZUhlYWRlcnM6IHtcbiAgICAgICAgJ21ldGhvZC5yZXNwb25zZS5oZWFkZXIuQWNjZXNzLUNvbnRyb2wtQWxsb3ctT3JpZ2luJzogXCInKidcIixcbiAgICAgICAgJ21ldGhvZC5yZXNwb25zZS5oZWFkZXIuQWNjZXNzLUNvbnRyb2wtQWxsb3ctSGVhZGVycyc6IFwiJ0NvbnRlbnQtVHlwZSxBdXRob3JpemF0aW9uLHgtcmVxdWVzdC1pZCx4LXNjaG9vbC1pZCx4LWFwaS1rZXknXCIsXG4gICAgICAgICdtZXRob2QucmVzcG9uc2UuaGVhZGVyLkFjY2Vzcy1Db250cm9sLUFsbG93LU1ldGhvZHMnOiBcIidHRVQsUE9TVCxQVVQsREVMRVRFLE9QVElPTlMsUEFUQ0gnXCIsXG4gICAgICB9LFxuICAgIH0pO1xuICAgIGFwaS5hZGRHYXRld2F5UmVzcG9uc2UoJ0RlZmF1bHQ1WFgnLCB7XG4gICAgICB0eXBlOiBhcGlnYXRld2F5LlJlc3BvbnNlVHlwZS5ERUZBVUxUXzVYWCxcbiAgICAgIHJlc3BvbnNlSGVhZGVyczoge1xuICAgICAgICAnbWV0aG9kLnJlc3BvbnNlLmhlYWRlci5BY2Nlc3MtQ29udHJvbC1BbGxvdy1PcmlnaW4nOiBcIicqJ1wiLFxuICAgICAgICAnbWV0aG9kLnJlc3BvbnNlLmhlYWRlci5BY2Nlc3MtQ29udHJvbC1BbGxvdy1IZWFkZXJzJzogXCInQ29udGVudC1UeXBlLEF1dGhvcml6YXRpb24seC1yZXF1ZXN0LWlkLHgtc2Nob29sLWlkLHgtYXBpLWtleSdcIixcbiAgICAgICAgJ21ldGhvZC5yZXNwb25zZS5oZWFkZXIuQWNjZXNzLUNvbnRyb2wtQWxsb3ctTWV0aG9kcyc6IFwiJ0dFVCxQT1NULFBVVCxERUxFVEUsT1BUSU9OUyxQQVRDSCdcIixcbiAgICAgIH0sXG4gICAgfSk7XG5cbiAgICAvLyBPdXRwdXRzXG4gICAgbmV3IGNkay5DZm5PdXRwdXQodGhpcywgJ0FwaVVybCcsIHtcbiAgICAgIHZhbHVlOiBhcGkudXJsLFxuICAgICAgZGVzY3JpcHRpb246IGAke3N0YWdlTmFtZX0gQVBJIEdhdGV3YXkgVVJMYCxcbiAgICAgIGV4cG9ydE5hbWU6IGBHb2RkYXJkJHtzdGFnZU5hbWV9QXBpVXJsYCxcbiAgICB9KTtcblxuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdMYW1iZGFGdW5jdGlvbk5hbWUnLCB7XG4gICAgICB2YWx1ZTogcnVzdExhbWJkYS5mdW5jdGlvbk5hbWUsXG4gICAgICBkZXNjcmlwdGlvbjogYCR7c3RhZ2VOYW1lfSBMYW1iZGEgRnVuY3Rpb24gTmFtZWAsXG4gICAgICBleHBvcnROYW1lOiBgR29kZGFyZCR7c3RhZ2VOYW1lfUxhbWJkYUZ1bmN0aW9uTmFtZWAsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnTGFtYmRhRnVuY3Rpb25Bcm4nLCB7XG4gICAgICB2YWx1ZTogcnVzdExhbWJkYS5mdW5jdGlvbkFybixcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IExhbWJkYSBGdW5jdGlvbiBBUk5gLFxuICAgICAgZXhwb3J0TmFtZTogYEdvZGRhcmQke3N0YWdlTmFtZX1MYW1iZGFGdW5jdGlvbkFybmAsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnTm90aWZpY2F0aW9uUHVzaFdvcmtlckZ1bmN0aW9uTmFtZScsIHtcbiAgICAgIHZhbHVlOiBub3RpZmljYXRpb25QdXNoV29ya2VyLmZ1bmN0aW9uTmFtZSxcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IEZDTSBvdXRib3ggd29ya2VyIGZ1bmN0aW9uIG5hbWVgLFxuICAgIH0pO1xuXG4gICAgbmV3IGNkay5DZm5PdXRwdXQodGhpcywgJ1VwbG9hZHNCdWNrZXROYW1lJywge1xuICAgICAgdmFsdWU6IHVwbG9hZHNCdWNrZXQuYnVja2V0TmFtZSxcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IFMzIFVwbG9hZHMgQnVja2V0IE5hbWVgLFxuICAgICAgZXhwb3J0TmFtZTogYEdvZGRhcmQke3N0YWdlTmFtZX1VcGxvYWRzQnVja2V0TmFtZWAsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnVXBsb2Fkc0J1Y2tldFVybCcsIHtcbiAgICAgIHZhbHVlOiBgaHR0cHM6Ly8ke3VwbG9hZHNCdWNrZXQuYnVja2V0UmVnaW9uYWxEb21haW5OYW1lfWAsXG4gICAgICBkZXNjcmlwdGlvbjogYCR7c3RhZ2VOYW1lfSBTMyBVcGxvYWRzIEJ1Y2tldCBCYXNlIFVSTGAsXG4gICAgICBleHBvcnROYW1lOiBgR29kZGFyZCR7c3RhZ2VOYW1lfVVwbG9hZHNCdWNrZXRVcmxgLFxuICAgIH0pO1xuICB9XG5cbiAgLyoqXG4gICAqIFRoZSBkYXRhYmFzZSBiYWNrdXAgaXMgZGVsaWJlcmF0ZWx5IGlzb2xhdGVkIGZyb20gdGhlIEFQSSBMYW1iZGEuIFRoZVxuICAgKiBTdXBhYmFzZSBDTEkgc3RhcnRzIHBnX2R1bXAgaW4gRG9ja2VyLCB3aGljaCBpcyBzdXBwb3J0ZWQgYnkgcHJpdmlsZWdlZFxuICAgKiBDb2RlQnVpbGQgYnV0IG5vdCBieSBMYW1iZGEuXG4gICAqL1xuICBwcml2YXRlIGFkZEJhY2t1cFBpcGVsaW5lKFxuICAgIGFwaTogYXBpZ2F0ZXdheS5SZXN0QXBpLFxuICAgIHVwbG9hZHNCdWNrZXQ6IHMzLklCdWNrZXQsXG4gICAgc3RhZ2U6ICdkZXYnIHwgJ3Byb2QnLFxuICApOiB2b2lkIHtcbiAgICBjb25zdCBzdGFnZU5hbWUgPSBzdGFnZS50b1VwcGVyQ2FzZSgpO1xuICAgIGNvbnN0IHN0YWdlSWQgPSBzdGFnZSA9PT0gJ2RldicgPyAnRGV2JyA6ICdQcm9kJztcbiAgICBjb25zdCByZXRlbnRpb25EYXlzID0gc3RhZ2UgPT09ICdwcm9kJyA/IDM2NSA6IDkwO1xuICAgIGNvbnN0IGJhY2t1cEtleSA9IG5ldyBrbXMuS2V5KHRoaXMsIGAke3N0YWdlSWR9QmFja3VwS2V5YCwge1xuICAgICAgYWxpYXM6IGBhbGlhcy9nb2RkYXJkLSR7c3RhZ2V9LWJhY2t1cHNgLFxuICAgICAgZW5hYmxlS2V5Um90YXRpb246IHRydWUsXG4gICAgICByZW1vdmFsUG9saWN5OiBjZGsuUmVtb3ZhbFBvbGljeS5SRVRBSU4sXG4gICAgfSk7XG5cbiAgICBjb25zdCBiYWNrdXBCdWNrZXQgPSBuZXcgczMuQnVja2V0KHRoaXMsIGAke3N0YWdlSWR9QmFja3VwQnVja2V0YCwge1xuICAgICAgYnVja2V0TmFtZTogY2RrLkZuLnN1YihgZ29kZGFyZC0ke3N0YWdlfS1iYWNrdXBzLVxcJHtBV1M6OkFjY291bnRJZH0tXFwke0FXUzo6UmVnaW9ufWApLFxuICAgICAgZW5jcnlwdGlvbjogczMuQnVja2V0RW5jcnlwdGlvbi5LTVMsXG4gICAgICBlbmNyeXB0aW9uS2V5OiBiYWNrdXBLZXksXG4gICAgICBidWNrZXRLZXlFbmFibGVkOiB0cnVlLFxuICAgICAgYmxvY2tQdWJsaWNBY2Nlc3M6IHMzLkJsb2NrUHVibGljQWNjZXNzLkJMT0NLX0FMTCxcbiAgICAgIGVuZm9yY2VTU0w6IHRydWUsXG4gICAgICB2ZXJzaW9uZWQ6IHRydWUsXG4gICAgICByZW1vdmFsUG9saWN5OiBjZGsuUmVtb3ZhbFBvbGljeS5SRVRBSU4sXG4gICAgICBsaWZlY3ljbGVSdWxlczogW3tcbiAgICAgICAgaWQ6IGBleHBpcmUtJHtzdGFnZX0tcmVjb3ZlcnktcG9pbnRzLWFmdGVyLSR7cmV0ZW50aW9uRGF5c30tZGF5c2AsXG4gICAgICAgIGVuYWJsZWQ6IHRydWUsXG4gICAgICAgIGV4cGlyYXRpb246IGNkay5EdXJhdGlvbi5kYXlzKHJldGVudGlvbkRheXMpLFxuICAgICAgICBub25jdXJyZW50VmVyc2lvbkV4cGlyYXRpb246IGNkay5EdXJhdGlvbi5kYXlzKDcpLFxuICAgICAgfV0sXG4gICAgfSk7XG5cbiAgICAvLyBDcmVhdGUgdGhpcyBzZWNyZXQgYmVmb3JlIGRlcGxveWluZyBhbmQgc3RvcmUgYSBKU09OIHZhbHVlIHdpdGggdGhlXG4gICAgLy8gYGRhdGFiYXNlX3VybGAga2V5LiBLZWVwaW5nIHRoZSB2YWx1ZSBvdXRzaWRlIENsb3VkRm9ybWF0aW9uIHByZXZlbnRzXG4gICAgLy8gZGF0YWJhc2UgY3JlZGVudGlhbHMgZnJvbSBhcHBlYXJpbmcgaW4gdGVtcGxhdGVzIG9yIGJ1aWxkIGxvZ3MuXG4gICAgY29uc3QgZGF0YWJhc2VTZWNyZXQgPSBzZWNyZXRzbWFuYWdlci5TZWNyZXQuZnJvbVNlY3JldE5hbWVWMihcbiAgICAgIHRoaXMsXG4gICAgICBgJHtzdGFnZUlkfVN1cGFiYXNlQmFja3VwRGF0YWJhc2VTZWNyZXRgLFxuICAgICAgYGdvZGRhcmQvJHtzdGFnZX0vc3VwYWJhc2UtYmFja3VwYCxcbiAgICApO1xuICAgIGNvbnN0IGRhdGFiYXNlU2VjcmV0TmFtZSA9IGBnb2RkYXJkLyR7c3RhZ2V9L3N1cGFiYXNlLWJhY2t1cGA7XG4gICAgY29uc3QgcHJvamVjdFJlZiA9IG5ldyBjZGsuQ2ZuUGFyYW1ldGVyKHRoaXMsIGAke3N0YWdlSWR9U3VwYWJhc2VQcm9qZWN0UmVmYCwge1xuICAgICAgdHlwZTogJ1N0cmluZycsXG4gICAgICBkZXNjcmlwdGlvbjogYCR7c3RhZ2VOYW1lfSBTdXBhYmFzZSBwcm9qZWN0IHJlZmVyZW5jZSByZWNvcmRlZCBpbiBlYWNoIGJhY2t1cCBtYW5pZmVzdC5gLFxuICAgIH0pO1xuXG4gICAgY29uc3Qgd29ya2VyU291cmNlID0gbmV3IHMzYXNzZXRzLkFzc2V0KHRoaXMsIGAke3N0YWdlSWR9QmFja3VwV29ya2VyU291cmNlYCwge1xuICAgICAgcGF0aDogcGF0aC5qb2luKF9fZGlybmFtZSwgJy4uLy4uL2JhY2t1cC93b3JrZXInKSxcbiAgICB9KTtcbiAgICBjb25zdCBiYWNrdXBQcm9qZWN0ID0gbmV3IGNvZGVidWlsZC5Qcm9qZWN0KHRoaXMsIGAke3N0YWdlSWR9U3VwYWJhc2VCYWNrdXBQcm9qZWN0YCwge1xuICAgICAgcHJvamVjdE5hbWU6IGBnb2RkYXJkLSR7c3RhZ2V9LXN1cGFiYXNlLWJhY2t1cGAsXG4gICAgICBkZXNjcmlwdGlvbjogYENyZWF0ZXMgZW5jcnlwdGVkIGxvZ2ljYWwgU3VwYWJhc2UgJHtzdGFnZU5hbWV9IHJlY292ZXJ5IGJ1bmRsZXMgaW4gUzMuYCxcbiAgICAgIHNvdXJjZTogY29kZWJ1aWxkLlNvdXJjZS5zMyh7XG4gICAgICAgIGJ1Y2tldDogd29ya2VyU291cmNlLmJ1Y2tldCxcbiAgICAgICAgcGF0aDogd29ya2VyU291cmNlLnMzT2JqZWN0S2V5LFxuICAgICAgfSksXG4gICAgICBidWlsZFNwZWM6IGNvZGVidWlsZC5CdWlsZFNwZWMuZnJvbVNvdXJjZUZpbGVuYW1lKCdidWlsZHNwZWMueW1sJyksXG4gICAgICBlbnZpcm9ubWVudDoge1xuICAgICAgICBidWlsZEltYWdlOiBjb2RlYnVpbGQuTGludXhCdWlsZEltYWdlLlNUQU5EQVJEXzdfMCxcbiAgICAgICAgcHJpdmlsZWdlZDogdHJ1ZSxcbiAgICAgICAgY29tcHV0ZVR5cGU6IGNvZGVidWlsZC5Db21wdXRlVHlwZS5NRURJVU0sXG4gICAgICAgIGVudmlyb25tZW50VmFyaWFibGVzOiB7XG4gICAgICAgICAgREFUQUJBU0VfVVJMOiB7XG4gICAgICAgICAgICB0eXBlOiBjb2RlYnVpbGQuQnVpbGRFbnZpcm9ubWVudFZhcmlhYmxlVHlwZS5TRUNSRVRTX01BTkFHRVIsXG4gICAgICAgICAgICAvLyBJbXBvcnRlZCBzZWNyZXRzIGhhdmUgYSBwYXJ0aWFsIEFSTiB3aXRob3V0IFNlY3JldHMgTWFuYWdlcidzXG4gICAgICAgICAgICAvLyByYW5kb20gc3VmZml4LiBDb2RlQnVpbGQgbXVzdCByZXNvbHZlIHRoaXMgYnkgc3RhYmxlIG5hbWUuXG4gICAgICAgICAgICB2YWx1ZTogYCR7ZGF0YWJhc2VTZWNyZXROYW1lfTpkYXRhYmFzZV91cmxgLFxuICAgICAgICAgIH0sXG4gICAgICAgICAgQkFDS1VQX0JVQ0tFVDogeyB2YWx1ZTogYmFja3VwQnVja2V0LmJ1Y2tldE5hbWUgfSxcbiAgICAgICAgICBVUExPQURTX0JVQ0tFVDogeyB2YWx1ZTogdXBsb2Fkc0J1Y2tldC5idWNrZXROYW1lIH0sXG4gICAgICAgICAgQkFDS1VQX0VOVklST05NRU5UOiB7IHZhbHVlOiBzdGFnZSB9LFxuICAgICAgICAgIFNVUEFCQVNFX1BST0pFQ1RfUkVGOiB7IHZhbHVlOiBwcm9qZWN0UmVmLnZhbHVlQXNTdHJpbmcgfSxcbiAgICAgICAgICBTVVBBQkFTRV9DTElfVkVSU0lPTjogeyB2YWx1ZTogJzIuNjcuMScgfSxcbiAgICAgICAgfSxcbiAgICAgIH0sXG4gICAgICB0aW1lb3V0OiBjZGsuRHVyYXRpb24uaG91cnMoMiksXG4gICAgICBxdWV1ZWRUaW1lb3V0OiBjZGsuRHVyYXRpb24ubWludXRlcygzMCksXG4gICAgICBjb25jdXJyZW50QnVpbGRMaW1pdDogMSxcbiAgICAgIGVuY3J5cHRpb25LZXk6IGJhY2t1cEtleSxcbiAgICAgIGxvZ2dpbmc6IHtcbiAgICAgICAgY2xvdWRXYXRjaDoge1xuICAgICAgICAgIGxvZ0dyb3VwOiBuZXcgbG9ncy5Mb2dHcm91cCh0aGlzLCBgJHtzdGFnZUlkfVN1cGFiYXNlQmFja3VwQnVpbGRMb2dHcm91cGAsIHtcbiAgICAgICAgICAgIHJldGVudGlvbjogbG9ncy5SZXRlbnRpb25EYXlzLk9ORV9NT05USCxcbiAgICAgICAgICAgIHJlbW92YWxQb2xpY3k6IGNkay5SZW1vdmFsUG9saWN5LlJFVEFJTixcbiAgICAgICAgICB9KSxcbiAgICAgICAgfSxcbiAgICAgIH0sXG4gICAgfSk7XG4gICAgZGF0YWJhc2VTZWNyZXQuZ3JhbnRSZWFkKGJhY2t1cFByb2plY3QpO1xuICAgIHdvcmtlclNvdXJjZS5ncmFudFJlYWQoYmFja3VwUHJvamVjdCk7XG4gICAgYmFja3VwQnVja2V0LmdyYW50UmVhZFdyaXRlKGJhY2t1cFByb2plY3QpO1xuICAgIHVwbG9hZHNCdWNrZXQuZ3JhbnRSZWFkKGJhY2t1cFByb2plY3QpO1xuXG4gICAgY29uc3Qgb3JjaGVzdHJhdG9yID0gbmV3IGxhbWJkYS5GdW5jdGlvbih0aGlzLCBgJHtzdGFnZUlkfUJhY2t1cE9yY2hlc3RyYXRvcmAsIHtcbiAgICAgIGZ1bmN0aW9uTmFtZTogYGdvZGRhcmQtJHtzdGFnZX0tYmFja3VwLW9yY2hlc3RyYXRvcmAsXG4gICAgICBydW50aW1lOiBsYW1iZGEuUnVudGltZS5QWVRIT05fM18xMixcbiAgICAgIGFyY2hpdGVjdHVyZTogbGFtYmRhLkFyY2hpdGVjdHVyZS5BUk1fNjQsXG4gICAgICBoYW5kbGVyOiAnYXBwLmhhbmRsZXInLFxuICAgICAgY29kZTogbGFtYmRhLkNvZGUuZnJvbUFzc2V0KHBhdGguam9pbihfX2Rpcm5hbWUsICcuLi8uLi9iYWNrdXAvb3JjaGVzdHJhdG9yJykpLFxuICAgICAgdGltZW91dDogY2RrLkR1cmF0aW9uLnNlY29uZHMoMzApLFxuICAgICAgbWVtb3J5U2l6ZTogMjU2LFxuICAgICAgZW52aXJvbm1lbnQ6IHsgQkFDS1VQX1BST0pFQ1RfTkFNRTogYmFja3VwUHJvamVjdC5wcm9qZWN0TmFtZSB9LFxuICAgICAgbG9nR3JvdXA6IG5ldyBsb2dzLkxvZ0dyb3VwKHRoaXMsIGAke3N0YWdlSWR9QmFja3VwT3JjaGVzdHJhdG9yTG9nR3JvdXBgLCB7XG4gICAgICAgIHJldGVudGlvbjogbG9ncy5SZXRlbnRpb25EYXlzLk9ORV9NT05USCxcbiAgICAgICAgcmVtb3ZhbFBvbGljeTogY2RrLlJlbW92YWxQb2xpY3kuUkVUQUlOLFxuICAgICAgfSksXG4gICAgfSk7XG4gICAgb3JjaGVzdHJhdG9yLmFkZFRvUm9sZVBvbGljeShuZXcgaWFtLlBvbGljeVN0YXRlbWVudCh7XG4gICAgICBhY3Rpb25zOiBbJ2NvZGVidWlsZDpTdGFydEJ1aWxkJ10sXG4gICAgICByZXNvdXJjZXM6IFtiYWNrdXBQcm9qZWN0LnByb2plY3RBcm5dLFxuICAgIH0pKTtcblxuICAgIGNvbnN0IG9wcyA9IGFwaS5yb290LmFkZFJlc291cmNlKCdvcHMnKTtcbiAgICBjb25zdCBiYWNrdXBzID0gb3BzLmFkZFJlc291cmNlKCdiYWNrdXBzJyk7XG4gICAgYmFja3Vwcy5hZGRNZXRob2QoJ1BPU1QnLCBuZXcgYXBpZ2F0ZXdheS5MYW1iZGFJbnRlZ3JhdGlvbihvcmNoZXN0cmF0b3IpLCB7XG4gICAgICBhdXRob3JpemF0aW9uVHlwZTogYXBpZ2F0ZXdheS5BdXRob3JpemF0aW9uVHlwZS5JQU0sXG4gICAgfSk7XG5cbiAgICBuZXcgY2xvdWR3YXRjaC5BbGFybSh0aGlzLCBgJHtzdGFnZUlkfUJhY2t1cEJ1aWxkRmFpbHVyZUFsYXJtYCwge1xuICAgICAgYWxhcm1EZXNjcmlwdGlvbjogYEEgJHtzdGFnZU5hbWV9IFN1cGFiYXNlIGJhY2t1cCBDb2RlQnVpbGQgam9iIGZhaWxlZC5gLFxuICAgICAgbWV0cmljOiBiYWNrdXBQcm9qZWN0Lm1ldHJpY0ZhaWxlZEJ1aWxkcyh7IHBlcmlvZDogY2RrLkR1cmF0aW9uLmRheXMoMSkgfSksXG4gICAgICB0aHJlc2hvbGQ6IDEsXG4gICAgICBldmFsdWF0aW9uUGVyaW9kczogMSxcbiAgICAgIHRyZWF0TWlzc2luZ0RhdGE6IGNsb3Vkd2F0Y2guVHJlYXRNaXNzaW5nRGF0YS5OT1RfQlJFQUNISU5HLFxuICAgIH0pO1xuICAgIG5ldyBjbG91ZHdhdGNoLkFsYXJtKHRoaXMsIGAke3N0YWdlSWR9QmFja3VwT3JjaGVzdHJhdG9yRXJyb3JBbGFybWAsIHtcbiAgICAgIGFsYXJtRGVzY3JpcHRpb246IGBUaGUgJHtzdGFnZU5hbWV9IFN1cGFiYXNlIGJhY2t1cCBvcmNoZXN0cmF0b3IgZmFpbGVkIHRvIHN0YXJ0IGEgYnVpbGQuYCxcbiAgICAgIG1ldHJpYzogb3JjaGVzdHJhdG9yLm1ldHJpY0Vycm9ycyh7IHBlcmlvZDogY2RrLkR1cmF0aW9uLmRheXMoMSkgfSksXG4gICAgICB0aHJlc2hvbGQ6IDEsXG4gICAgICBldmFsdWF0aW9uUGVyaW9kczogMSxcbiAgICAgIHRyZWF0TWlzc2luZ0RhdGE6IGNsb3Vkd2F0Y2guVHJlYXRNaXNzaW5nRGF0YS5OT1RfQlJFQUNISU5HLFxuICAgIH0pO1xuXG4gICAgbmV3IGNkay5DZm5PdXRwdXQodGhpcywgJ0JhY2t1cEJ1Y2tldE5hbWUnLCB7IHZhbHVlOiBiYWNrdXBCdWNrZXQuYnVja2V0TmFtZSB9KTtcbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnQmFja3VwQXBpUGF0aCcsIHtcbiAgICAgIHZhbHVlOiBgJHthcGkudXJsfW9wcy9iYWNrdXBzYCxcbiAgICAgIGRlc2NyaXB0aW9uOiBgSUFNLWF1dGhlbnRpY2F0ZWQgZW5kcG9pbnQgdG8gbWFudWFsbHkgc3RhcnQgYSAke3N0YWdlTmFtZX0gYmFja3VwLmAsXG4gICAgfSk7XG4gICAgbmV3IGNkay5DZm5PdXRwdXQodGhpcywgJ0JhY2t1cEFwaUludm9rZUFybicsIHtcbiAgICAgIHZhbHVlOiBhcGkuYXJuRm9yRXhlY3V0ZUFwaSgnUE9TVCcsICcvb3BzL2JhY2t1cHMnLCAnKicpLFxuICAgICAgZGVzY3JpcHRpb246IGBJQU0gcmVzb3VyY2UgQVJOIGZvciBpbnZva2luZyB0aGUgJHtzdGFnZU5hbWV9IGJhY2t1cCBlbmRwb2ludC5gLFxuICAgIH0pO1xuICB9XG59XG4iXX0=