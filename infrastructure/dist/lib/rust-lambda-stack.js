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
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoicnVzdC1sYW1iZGEtc3RhY2suanMiLCJzb3VyY2VSb290IjoiIiwic291cmNlcyI6WyIuLi8uLi9saWIvcnVzdC1sYW1iZGEtc3RhY2sudHMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6Ijs7O0FBQUEsbUNBQW1DO0FBQ25DLGlEQUFpRDtBQUNqRCx5REFBeUQ7QUFDekQsNkNBQTZDO0FBQzdDLHlDQUF5QztBQUN6QyxzREFBc0Q7QUFDdEQsdURBQXVEO0FBQ3ZELHlEQUF5RDtBQUN6RCxpREFBaUQ7QUFDakQsMERBQTBEO0FBQzFELDJDQUEyQztBQUMzQywyQ0FBMkM7QUFDM0MsaUVBQWlFO0FBQ2pFLDZCQUE2QjtBQU83QixNQUFhLGVBQWdCLFNBQVEsR0FBRyxDQUFDLEtBQUs7SUFDNUMsWUFBWSxLQUFnQixFQUFFLEVBQVUsRUFBRSxLQUF1QjtRQUMvRCxLQUFLLENBQUMsS0FBSyxFQUFFLEVBQUUsRUFBRSxLQUFLLENBQUMsQ0FBQztRQUV4QixNQUFNLEVBQUUsS0FBSyxFQUFFLEdBQUcsS0FBSyxDQUFDO1FBQ3hCLE1BQU0sU0FBUyxHQUFHLEtBQUssQ0FBQyxXQUFXLEVBQUUsQ0FBQztRQUV0QyxzQ0FBc0M7UUFDdEMsTUFBTSxhQUFhLEdBQUcsSUFBSSxFQUFFLENBQUMsTUFBTSxDQUFDLElBQUksRUFBRSxVQUFVLFNBQVMsZUFBZSxFQUFFO1lBQzVFLFVBQVUsRUFBRSxtQkFBbUIsS0FBSyxFQUFFO1lBQ3RDLGdCQUFnQixFQUFFLElBQUk7WUFDdEIsaUJBQWlCLEVBQUUsRUFBRSxDQUFDLGlCQUFpQixDQUFDLFVBQVU7WUFDbEQsSUFBSSxFQUFFO2dCQUNKO29CQUNFLGNBQWMsRUFBRSxDQUFDLEVBQUUsQ0FBQyxXQUFXLENBQUMsR0FBRyxFQUFFLEVBQUUsQ0FBQyxXQUFXLENBQUMsR0FBRyxDQUFDO29CQUN4RCxjQUFjLEVBQUUsQ0FBQyxHQUFHLENBQUM7b0JBQ3JCLGNBQWMsRUFBRSxDQUFDLEdBQUcsQ0FBQztpQkFDdEI7YUFDRjtZQUNELGFBQWEsRUFBRSxHQUFHLENBQUMsYUFBYSxDQUFDLE1BQU07WUFDdkMsU0FBUyxFQUFFLElBQUk7U0FDaEIsQ0FBQyxDQUFDO1FBRUgsZ0NBQWdDO1FBQ2hDLDZGQUE2RjtRQUM3RixrSEFBa0g7UUFDbEgsTUFBTSxVQUFVLEdBQUcsSUFBSSxNQUFNLENBQUMsUUFBUSxDQUFDLElBQUksRUFBRSxVQUFVLFNBQVMsUUFBUSxFQUFFO1lBQ3hFLFlBQVksRUFBRSxXQUFXLEtBQUssRUFBRTtZQUNoQyxPQUFPLEVBQUUsTUFBTSxDQUFDLE9BQU8sQ0FBQyxlQUFlLEVBQUUsbUNBQW1DO1lBQzVFLFlBQVksRUFBRSxNQUFNLENBQUMsWUFBWSxDQUFDLE1BQU0sRUFBRSxrQ0FBa0M7WUFDNUUsT0FBTyxFQUFFLFdBQVc7WUFDcEIsSUFBSSxFQUFFLE1BQU0sQ0FBQyxJQUFJLENBQUMsU0FBUyxDQUFDLElBQUksQ0FBQyxJQUFJLENBQUMsU0FBUyxFQUFFLG9EQUFvRCxDQUFDLEVBQUU7Z0JBQ3RHLE9BQU8sRUFBRSxDQUFDLElBQUksRUFBRSxZQUFZLENBQUM7YUFDOUIsQ0FBQztZQUNGLFVBQVUsRUFBRSxLQUFLLEtBQUssS0FBSyxDQUFDLENBQUMsQ0FBQyxHQUFHLENBQUMsQ0FBQyxDQUFDLEdBQUc7WUFDdkMsT0FBTyxFQUFFLEdBQUcsQ0FBQyxRQUFRLENBQUMsT0FBTyxDQUFDLEVBQUUsQ0FBQztZQUNqQyxXQUFXLEVBQUU7Z0JBQ1gsUUFBUSxFQUFFLE1BQU07Z0JBQ2hCLGdCQUFnQixFQUFFLGFBQWEsQ0FBQyxVQUFVO2dCQUMxQyxXQUFXLEVBQUUsV0FBVyxhQUFhLENBQUMsd0JBQXdCLEVBQUU7YUFDakU7WUFDRCxRQUFRLEVBQUUsSUFBSSxJQUFJLENBQUMsUUFBUSxDQUFDLElBQUksRUFBRSxVQUFVLFNBQVMsZ0JBQWdCLEVBQUU7Z0JBQ3JFLFlBQVksRUFBRSx1QkFBdUIsS0FBSyxFQUFFO2dCQUM1QyxTQUFTLEVBQUUsSUFBSSxDQUFDLGFBQWEsQ0FBQyxRQUFRO2dCQUN0QyxhQUFhLEVBQUUsR0FBRyxDQUFDLGFBQWEsQ0FBQyxPQUFPO2FBQ3pDLENBQUM7WUFDRixXQUFXLEVBQUUsV0FBVyxTQUFTLCtDQUErQztTQUNqRixDQUFDLENBQUM7UUFFSCxrREFBa0Q7UUFDbEQsYUFBYSxDQUFDLFFBQVEsQ0FBQyxVQUFVLENBQUMsQ0FBQztRQUVuQywwRUFBMEU7UUFDMUUsMkVBQTJFO1FBQzNFLDhDQUE4QztRQUM5QyxNQUFNLHNCQUFzQixHQUFHLElBQUksTUFBTSxDQUFDLFFBQVEsQ0FBQyxJQUFJLEVBQUUsVUFBVSxTQUFTLHdCQUF3QixFQUFFO1lBQ3BHLFlBQVksRUFBRSxXQUFXLEtBQUssMkJBQTJCO1lBQ3pELE9BQU8sRUFBRSxNQUFNLENBQUMsT0FBTyxDQUFDLGVBQWU7WUFDdkMsWUFBWSxFQUFFLE1BQU0sQ0FBQyxZQUFZLENBQUMsTUFBTTtZQUN4QyxPQUFPLEVBQUUsV0FBVztZQUNwQixJQUFJLEVBQUUsTUFBTSxDQUFDLElBQUksQ0FBQyxTQUFTLENBQUMsSUFBSSxDQUFDLElBQUksQ0FBQyxTQUFTLEVBQUUsNkRBQTZELENBQUMsRUFBRTtnQkFDL0csT0FBTyxFQUFFLENBQUMsSUFBSSxFQUFFLFlBQVksQ0FBQzthQUM5QixDQUFDO1lBQ0YsVUFBVSxFQUFFLEdBQUc7WUFDZixPQUFPLEVBQUUsR0FBRyxDQUFDLFFBQVEsQ0FBQyxPQUFPLENBQUMsRUFBRSxDQUFDO1lBQ2pDLFdBQVcsRUFBRSxFQUFFLFFBQVEsRUFBRSxNQUFNLEVBQUU7WUFDakMsUUFBUSxFQUFFLElBQUksSUFBSSxDQUFDLFFBQVEsQ0FBQyxJQUFJLEVBQUUsVUFBVSxTQUFTLGdDQUFnQyxFQUFFO2dCQUNyRixZQUFZLEVBQUUsdUJBQXVCLEtBQUssMkJBQTJCO2dCQUNyRSxTQUFTLEVBQUUsSUFBSSxDQUFDLGFBQWEsQ0FBQyxRQUFRO2dCQUN0QyxhQUFhLEVBQUUsR0FBRyxDQUFDLGFBQWEsQ0FBQyxPQUFPO2FBQ3pDLENBQUM7WUFDRixXQUFXLEVBQUUsV0FBVyxTQUFTLG9DQUFvQztTQUN0RSxDQUFDLENBQUM7UUFDSCx5RUFBeUU7UUFDekUsZ0RBQWdEO1FBQ2hELHNCQUFzQixDQUFDLFdBQVcsQ0FBQyxVQUFVLENBQUMsQ0FBQztRQUMvQyxJQUFJLE1BQU0sQ0FBQyxJQUFJLENBQUMsSUFBSSxFQUFFLFVBQVUsU0FBUywwQkFBMEIsRUFBRTtZQUNuRSxXQUFXLEVBQUUsa0JBQWtCLFNBQVMsOEJBQThCO1lBQ3RFLFFBQVEsRUFBRSxNQUFNLENBQUMsUUFBUSxDQUFDLElBQUksQ0FBQyxHQUFHLENBQUMsUUFBUSxDQUFDLE9BQU8sQ0FBQyxDQUFDLENBQUMsQ0FBQztZQUN2RCxPQUFPLEVBQUUsQ0FBQyxJQUFJLE9BQU8sQ0FBQyxjQUFjLENBQUMsc0JBQXNCLENBQUMsQ0FBQztTQUM5RCxDQUFDLENBQUM7UUFHSCxjQUFjO1FBQ2QsTUFBTSxHQUFHLEdBQUcsSUFBSSxVQUFVLENBQUMsT0FBTyxDQUFDLElBQUksRUFBRSxVQUFVLFNBQVMsS0FBSyxFQUFFO1lBQ2pFLFdBQVcsRUFBRSxXQUFXLFNBQVMsTUFBTTtZQUN2QyxXQUFXLEVBQUUsR0FBRyxTQUFTLGtEQUFrRDtZQUMzRSxnQkFBZ0IsRUFBRSxDQUFDLEtBQUssQ0FBQztZQUN6QixhQUFhLEVBQUU7Z0JBQ2IsU0FBUyxFQUFFLEtBQUs7Z0JBQ2hCLGNBQWMsRUFBRSxLQUFLLEtBQUssTUFBTTtnQkFDaEMsY0FBYyxFQUFFLElBQUk7YUFDckI7WUFDRCwyREFBMkQ7WUFDM0Qsa0VBQWtFO1lBQ2xFLHlFQUF5RTtZQUN6RSw4RUFBOEU7U0FDL0UsQ0FBQyxDQUFDO1FBRUgsZ0NBQWdDO1FBQ2hDLE1BQU0saUJBQWlCLEdBQUcsSUFBSSxVQUFVLENBQUMsaUJBQWlCLENBQUMsVUFBVSxFQUFFO1lBQ3JFLEtBQUssRUFBRSxJQUFJO1NBQ1osQ0FBQyxDQUFDO1FBRUgsbUJBQW1CO1FBQ25CLEdBQUcsQ0FBQyxJQUFJLENBQUMsU0FBUyxDQUFDLEtBQUssRUFBRSxpQkFBaUIsQ0FBQyxDQUFDO1FBQzdDLHNFQUFzRTtRQUN0RSxHQUFHLENBQUMsSUFBSSxDQUFDLFNBQVMsQ0FBQyxTQUFTLEVBQUUsaUJBQWlCLENBQUMsQ0FBQztRQUVqRCw0Q0FBNEM7UUFDNUMsTUFBTSxhQUFhLEdBQUcsR0FBRyxDQUFDLElBQUksQ0FBQyxXQUFXLENBQUMsVUFBVSxDQUFDLENBQUM7UUFDdkQsYUFBYSxDQUFDLFNBQVMsQ0FBQyxLQUFLLEVBQUUsaUJBQWlCLENBQUMsQ0FBQztRQUNsRCxrRUFBa0U7UUFDbEUsYUFBYSxDQUFDLFNBQVMsQ0FBQyxTQUFTLEVBQUUsaUJBQWlCLENBQUMsQ0FBQztRQUV0RCxJQUFJLENBQUMsaUJBQWlCLENBQUMsR0FBRyxFQUFFLGFBQWEsRUFBRSxLQUFLLENBQUMsQ0FBQztRQUVsRCxrRUFBa0U7UUFDbEUsMkVBQTJFO1FBQzNFLEdBQUcsQ0FBQyxrQkFBa0IsQ0FBQyxZQUFZLEVBQUU7WUFDbkMsSUFBSSxFQUFFLFVBQVUsQ0FBQyxZQUFZLENBQUMsV0FBVztZQUN6QyxlQUFlLEVBQUU7Z0JBQ2Ysb0RBQW9ELEVBQUUsS0FBSztnQkFDM0QscURBQXFELEVBQUUsaUVBQWlFO2dCQUN4SCxxREFBcUQsRUFBRSxxQ0FBcUM7YUFDN0Y7U0FDRixDQUFDLENBQUM7UUFDSCxHQUFHLENBQUMsa0JBQWtCLENBQUMsWUFBWSxFQUFFO1lBQ25DLElBQUksRUFBRSxVQUFVLENBQUMsWUFBWSxDQUFDLFdBQVc7WUFDekMsZUFBZSxFQUFFO2dCQUNmLG9EQUFvRCxFQUFFLEtBQUs7Z0JBQzNELHFEQUFxRCxFQUFFLGlFQUFpRTtnQkFDeEgscURBQXFELEVBQUUscUNBQXFDO2FBQzdGO1NBQ0YsQ0FBQyxDQUFDO1FBRUgsVUFBVTtRQUNWLElBQUksR0FBRyxDQUFDLFNBQVMsQ0FBQyxJQUFJLEVBQUUsUUFBUSxFQUFFO1lBQ2hDLEtBQUssRUFBRSxHQUFHLENBQUMsR0FBRztZQUNkLFdBQVcsRUFBRSxHQUFHLFNBQVMsa0JBQWtCO1lBQzNDLFVBQVUsRUFBRSxVQUFVLFNBQVMsUUFBUTtTQUN4QyxDQUFDLENBQUM7UUFFSCxJQUFJLEdBQUcsQ0FBQyxTQUFTLENBQUMsSUFBSSxFQUFFLG9CQUFvQixFQUFFO1lBQzVDLEtBQUssRUFBRSxVQUFVLENBQUMsWUFBWTtZQUM5QixXQUFXLEVBQUUsR0FBRyxTQUFTLHVCQUF1QjtZQUNoRCxVQUFVLEVBQUUsVUFBVSxTQUFTLG9CQUFvQjtTQUNwRCxDQUFDLENBQUM7UUFFSCxJQUFJLEdBQUcsQ0FBQyxTQUFTLENBQUMsSUFBSSxFQUFFLG1CQUFtQixFQUFFO1lBQzNDLEtBQUssRUFBRSxVQUFVLENBQUMsV0FBVztZQUM3QixXQUFXLEVBQUUsR0FBRyxTQUFTLHNCQUFzQjtZQUMvQyxVQUFVLEVBQUUsVUFBVSxTQUFTLG1CQUFtQjtTQUNuRCxDQUFDLENBQUM7UUFFSCxJQUFJLEdBQUcsQ0FBQyxTQUFTLENBQUMsSUFBSSxFQUFFLG9DQUFvQyxFQUFFO1lBQzVELEtBQUssRUFBRSxzQkFBc0IsQ0FBQyxZQUFZO1lBQzFDLFdBQVcsRUFBRSxHQUFHLFNBQVMsa0NBQWtDO1NBQzVELENBQUMsQ0FBQztRQUVILElBQUksR0FBRyxDQUFDLFNBQVMsQ0FBQyxJQUFJLEVBQUUsbUJBQW1CLEVBQUU7WUFDM0MsS0FBSyxFQUFFLGFBQWEsQ0FBQyxVQUFVO1lBQy9CLFdBQVcsRUFBRSxHQUFHLFNBQVMseUJBQXlCO1lBQ2xELFVBQVUsRUFBRSxVQUFVLFNBQVMsbUJBQW1CO1NBQ25ELENBQUMsQ0FBQztRQUVILElBQUksR0FBRyxDQUFDLFNBQVMsQ0FBQyxJQUFJLEVBQUUsa0JBQWtCLEVBQUU7WUFDMUMsS0FBSyxFQUFFLFdBQVcsYUFBYSxDQUFDLHdCQUF3QixFQUFFO1lBQzFELFdBQVcsRUFBRSxHQUFHLFNBQVMsNkJBQTZCO1lBQ3RELFVBQVUsRUFBRSxVQUFVLFNBQVMsa0JBQWtCO1NBQ2xELENBQUMsQ0FBQztJQUNMLENBQUM7SUFFRDs7OztPQUlHO0lBQ0ssaUJBQWlCLENBQ3ZCLEdBQXVCLEVBQ3ZCLGFBQXlCLEVBQ3pCLEtBQXFCO1FBRXJCLE1BQU0sU0FBUyxHQUFHLEtBQUssQ0FBQyxXQUFXLEVBQUUsQ0FBQztRQUN0QyxNQUFNLE9BQU8sR0FBRyxLQUFLLEtBQUssS0FBSyxDQUFDLENBQUMsQ0FBQyxLQUFLLENBQUMsQ0FBQyxDQUFDLE1BQU0sQ0FBQztRQUNqRCxNQUFNLGFBQWEsR0FBRyxLQUFLLEtBQUssTUFBTSxDQUFDLENBQUMsQ0FBQyxHQUFHLENBQUMsQ0FBQyxDQUFDLEVBQUUsQ0FBQztRQUNsRCxNQUFNLFNBQVMsR0FBRyxJQUFJLEdBQUcsQ0FBQyxHQUFHLENBQUMsSUFBSSxFQUFFLEdBQUcsT0FBTyxXQUFXLEVBQUU7WUFDekQsS0FBSyxFQUFFLGlCQUFpQixLQUFLLFVBQVU7WUFDdkMsaUJBQWlCLEVBQUUsSUFBSTtZQUN2QixhQUFhLEVBQUUsR0FBRyxDQUFDLGFBQWEsQ0FBQyxNQUFNO1NBQ3hDLENBQUMsQ0FBQztRQUVILE1BQU0sWUFBWSxHQUFHLElBQUksRUFBRSxDQUFDLE1BQU0sQ0FBQyxJQUFJLEVBQUUsR0FBRyxPQUFPLGNBQWMsRUFBRTtZQUNqRSxVQUFVLEVBQUUsR0FBRyxDQUFDLEVBQUUsQ0FBQyxHQUFHLENBQUMsV0FBVyxLQUFLLDZDQUE2QyxDQUFDO1lBQ3JGLFVBQVUsRUFBRSxFQUFFLENBQUMsZ0JBQWdCLENBQUMsR0FBRztZQUNuQyxhQUFhLEVBQUUsU0FBUztZQUN4QixnQkFBZ0IsRUFBRSxJQUFJO1lBQ3RCLGlCQUFpQixFQUFFLEVBQUUsQ0FBQyxpQkFBaUIsQ0FBQyxTQUFTO1lBQ2pELFVBQVUsRUFBRSxJQUFJO1lBQ2hCLFNBQVMsRUFBRSxJQUFJO1lBQ2YsYUFBYSxFQUFFLEdBQUcsQ0FBQyxhQUFhLENBQUMsTUFBTTtZQUN2QyxjQUFjLEVBQUUsQ0FBQztvQkFDZixFQUFFLEVBQUUsVUFBVSxLQUFLLDBCQUEwQixhQUFhLE9BQU87b0JBQ2pFLE9BQU8sRUFBRSxJQUFJO29CQUNiLFVBQVUsRUFBRSxHQUFHLENBQUMsUUFBUSxDQUFDLElBQUksQ0FBQyxhQUFhLENBQUM7b0JBQzVDLDJCQUEyQixFQUFFLEdBQUcsQ0FBQyxRQUFRLENBQUMsSUFBSSxDQUFDLENBQUMsQ0FBQztpQkFDbEQsQ0FBQztTQUNILENBQUMsQ0FBQztRQUVILHNFQUFzRTtRQUN0RSx3RUFBd0U7UUFDeEUsa0VBQWtFO1FBQ2xFLE1BQU0sY0FBYyxHQUFHLGNBQWMsQ0FBQyxNQUFNLENBQUMsZ0JBQWdCLENBQzNELElBQUksRUFDSixHQUFHLE9BQU8sOEJBQThCLEVBQ3hDLFdBQVcsS0FBSyxrQkFBa0IsQ0FDbkMsQ0FBQztRQUNGLE1BQU0sa0JBQWtCLEdBQUcsV0FBVyxLQUFLLGtCQUFrQixDQUFDO1FBQzlELE1BQU0sVUFBVSxHQUFHLElBQUksR0FBRyxDQUFDLFlBQVksQ0FBQyxJQUFJLEVBQUUsR0FBRyxPQUFPLG9CQUFvQixFQUFFO1lBQzVFLElBQUksRUFBRSxRQUFRO1lBQ2QsV0FBVyxFQUFFLEdBQUcsU0FBUywrREFBK0Q7U0FDekYsQ0FBQyxDQUFDO1FBRUgsTUFBTSxZQUFZLEdBQUcsSUFBSSxRQUFRLENBQUMsS0FBSyxDQUFDLElBQUksRUFBRSxHQUFHLE9BQU8sb0JBQW9CLEVBQUU7WUFDNUUsSUFBSSxFQUFFLElBQUksQ0FBQyxJQUFJLENBQUMsU0FBUyxFQUFFLHFCQUFxQixDQUFDO1NBQ2xELENBQUMsQ0FBQztRQUNILE1BQU0sYUFBYSxHQUFHLElBQUksU0FBUyxDQUFDLE9BQU8sQ0FBQyxJQUFJLEVBQUUsR0FBRyxPQUFPLHVCQUF1QixFQUFFO1lBQ25GLFdBQVcsRUFBRSxXQUFXLEtBQUssa0JBQWtCO1lBQy9DLFdBQVcsRUFBRSxzQ0FBc0MsU0FBUywwQkFBMEI7WUFDdEYsTUFBTSxFQUFFLFNBQVMsQ0FBQyxNQUFNLENBQUMsRUFBRSxDQUFDO2dCQUMxQixNQUFNLEVBQUUsWUFBWSxDQUFDLE1BQU07Z0JBQzNCLElBQUksRUFBRSxZQUFZLENBQUMsV0FBVzthQUMvQixDQUFDO1lBQ0YsU0FBUyxFQUFFLFNBQVMsQ0FBQyxTQUFTLENBQUMsa0JBQWtCLENBQUMsZUFBZSxDQUFDO1lBQ2xFLFdBQVcsRUFBRTtnQkFDWCxVQUFVLEVBQUUsU0FBUyxDQUFDLGVBQWUsQ0FBQyxZQUFZO2dCQUNsRCxVQUFVLEVBQUUsSUFBSTtnQkFDaEIsV0FBVyxFQUFFLFNBQVMsQ0FBQyxXQUFXLENBQUMsTUFBTTtnQkFDekMsb0JBQW9CLEVBQUU7b0JBQ3BCLFlBQVksRUFBRTt3QkFDWixJQUFJLEVBQUUsU0FBUyxDQUFDLDRCQUE0QixDQUFDLGVBQWU7d0JBQzVELGdFQUFnRTt3QkFDaEUsNkRBQTZEO3dCQUM3RCxLQUFLLEVBQUUsR0FBRyxrQkFBa0IsZUFBZTtxQkFDNUM7b0JBQ0QsYUFBYSxFQUFFLEVBQUUsS0FBSyxFQUFFLFlBQVksQ0FBQyxVQUFVLEVBQUU7b0JBQ2pELGNBQWMsRUFBRSxFQUFFLEtBQUssRUFBRSxhQUFhLENBQUMsVUFBVSxFQUFFO29CQUNuRCxrQkFBa0IsRUFBRSxFQUFFLEtBQUssRUFBRSxLQUFLLEVBQUU7b0JBQ3BDLG9CQUFvQixFQUFFLEVBQUUsS0FBSyxFQUFFLFVBQVUsQ0FBQyxhQUFhLEVBQUU7b0JBQ3pELG9CQUFvQixFQUFFLEVBQUUsS0FBSyxFQUFFLFFBQVEsRUFBRTtpQkFDMUM7YUFDRjtZQUNELE9BQU8sRUFBRSxHQUFHLENBQUMsUUFBUSxDQUFDLEtBQUssQ0FBQyxDQUFDLENBQUM7WUFDOUIsYUFBYSxFQUFFLEdBQUcsQ0FBQyxRQUFRLENBQUMsT0FBTyxDQUFDLEVBQUUsQ0FBQztZQUN2QyxvQkFBb0IsRUFBRSxDQUFDO1lBQ3ZCLGFBQWEsRUFBRSxTQUFTO1lBQ3hCLE9BQU8sRUFBRTtnQkFDUCxVQUFVLEVBQUU7b0JBQ1YsUUFBUSxFQUFFLElBQUksSUFBSSxDQUFDLFFBQVEsQ0FBQyxJQUFJLEVBQUUsR0FBRyxPQUFPLDZCQUE2QixFQUFFO3dCQUN6RSxTQUFTLEVBQUUsSUFBSSxDQUFDLGFBQWEsQ0FBQyxTQUFTO3dCQUN2QyxhQUFhLEVBQUUsR0FBRyxDQUFDLGFBQWEsQ0FBQyxNQUFNO3FCQUN4QyxDQUFDO2lCQUNIO2FBQ0Y7U0FDRixDQUFDLENBQUM7UUFDSCxjQUFjLENBQUMsU0FBUyxDQUFDLGFBQWEsQ0FBQyxDQUFDO1FBQ3hDLFlBQVksQ0FBQyxTQUFTLENBQUMsYUFBYSxDQUFDLENBQUM7UUFDdEMsWUFBWSxDQUFDLGNBQWMsQ0FBQyxhQUFhLENBQUMsQ0FBQztRQUMzQyxhQUFhLENBQUMsU0FBUyxDQUFDLGFBQWEsQ0FBQyxDQUFDO1FBRXZDLE1BQU0sWUFBWSxHQUFHLElBQUksTUFBTSxDQUFDLFFBQVEsQ0FBQyxJQUFJLEVBQUUsR0FBRyxPQUFPLG9CQUFvQixFQUFFO1lBQzdFLFlBQVksRUFBRSxXQUFXLEtBQUssc0JBQXNCO1lBQ3BELE9BQU8sRUFBRSxNQUFNLENBQUMsT0FBTyxDQUFDLFdBQVc7WUFDbkMsWUFBWSxFQUFFLE1BQU0sQ0FBQyxZQUFZLENBQUMsTUFBTTtZQUN4QyxPQUFPLEVBQUUsYUFBYTtZQUN0QixJQUFJLEVBQUUsTUFBTSxDQUFDLElBQUksQ0FBQyxTQUFTLENBQUMsSUFBSSxDQUFDLElBQUksQ0FBQyxTQUFTLEVBQUUsMkJBQTJCLENBQUMsQ0FBQztZQUM5RSxPQUFPLEVBQUUsR0FBRyxDQUFDLFFBQVEsQ0FBQyxPQUFPLENBQUMsRUFBRSxDQUFDO1lBQ2pDLFVBQVUsRUFBRSxHQUFHO1lBQ2YsV0FBVyxFQUFFLEVBQUUsbUJBQW1CLEVBQUUsYUFBYSxDQUFDLFdBQVcsRUFBRTtZQUMvRCxRQUFRLEVBQUUsSUFBSSxJQUFJLENBQUMsUUFBUSxDQUFDLElBQUksRUFBRSxHQUFHLE9BQU8sNEJBQTRCLEVBQUU7Z0JBQ3hFLFNBQVMsRUFBRSxJQUFJLENBQUMsYUFBYSxDQUFDLFNBQVM7Z0JBQ3ZDLGFBQWEsRUFBRSxHQUFHLENBQUMsYUFBYSxDQUFDLE1BQU07YUFDeEMsQ0FBQztTQUNILENBQUMsQ0FBQztRQUNILFlBQVksQ0FBQyxlQUFlLENBQUMsSUFBSSxHQUFHLENBQUMsZUFBZSxDQUFDO1lBQ25ELE9BQU8sRUFBRSxDQUFDLHNCQUFzQixDQUFDO1lBQ2pDLFNBQVMsRUFBRSxDQUFDLGFBQWEsQ0FBQyxVQUFVLENBQUM7U0FDdEMsQ0FBQyxDQUFDLENBQUM7UUFFSixNQUFNLEdBQUcsR0FBRyxHQUFHLENBQUMsSUFBSSxDQUFDLFdBQVcsQ0FBQyxLQUFLLENBQUMsQ0FBQztRQUN4QyxNQUFNLE9BQU8sR0FBRyxHQUFHLENBQUMsV0FBVyxDQUFDLFNBQVMsQ0FBQyxDQUFDO1FBQzNDLE9BQU8sQ0FBQyxTQUFTLENBQUMsTUFBTSxFQUFFLElBQUksVUFBVSxDQUFDLGlCQUFpQixDQUFDLFlBQVksQ0FBQyxFQUFFO1lBQ3hFLGlCQUFpQixFQUFFLFVBQVUsQ0FBQyxpQkFBaUIsQ0FBQyxHQUFHO1NBQ3BELENBQUMsQ0FBQztRQUVILElBQUksVUFBVSxDQUFDLEtBQUssQ0FBQyxJQUFJLEVBQUUsR0FBRyxPQUFPLHlCQUF5QixFQUFFO1lBQzlELGdCQUFnQixFQUFFLEtBQUssU0FBUyx3Q0FBd0M7WUFDeEUsTUFBTSxFQUFFLGFBQWEsQ0FBQyxrQkFBa0IsQ0FBQyxFQUFFLE1BQU0sRUFBRSxHQUFHLENBQUMsUUFBUSxDQUFDLElBQUksQ0FBQyxDQUFDLENBQUMsRUFBRSxDQUFDO1lBQzFFLFNBQVMsRUFBRSxDQUFDO1lBQ1osaUJBQWlCLEVBQUUsQ0FBQztZQUNwQixnQkFBZ0IsRUFBRSxVQUFVLENBQUMsZ0JBQWdCLENBQUMsYUFBYTtTQUM1RCxDQUFDLENBQUM7UUFDSCxJQUFJLFVBQVUsQ0FBQyxLQUFLLENBQUMsSUFBSSxFQUFFLEdBQUcsT0FBTyw4QkFBOEIsRUFBRTtZQUNuRSxnQkFBZ0IsRUFBRSxPQUFPLFNBQVMsd0RBQXdEO1lBQzFGLE1BQU0sRUFBRSxZQUFZLENBQUMsWUFBWSxDQUFDLEVBQUUsTUFBTSxFQUFFLEdBQUcsQ0FBQyxRQUFRLENBQUMsSUFBSSxDQUFDLENBQUMsQ0FBQyxFQUFFLENBQUM7WUFDbkUsU0FBUyxFQUFFLENBQUM7WUFDWixpQkFBaUIsRUFBRSxDQUFDO1lBQ3BCLGdCQUFnQixFQUFFLFVBQVUsQ0FBQyxnQkFBZ0IsQ0FBQyxhQUFhO1NBQzVELENBQUMsQ0FBQztRQUVILElBQUksR0FBRyxDQUFDLFNBQVMsQ0FBQyxJQUFJLEVBQUUsa0JBQWtCLEVBQUUsRUFBRSxLQUFLLEVBQUUsWUFBWSxDQUFDLFVBQVUsRUFBRSxDQUFDLENBQUM7UUFDaEYsSUFBSSxHQUFHLENBQUMsU0FBUyxDQUFDLElBQUksRUFBRSxlQUFlLEVBQUU7WUFDdkMsS0FBSyxFQUFFLEdBQUcsR0FBRyxDQUFDLEdBQUcsYUFBYTtZQUM5QixXQUFXLEVBQUUsa0RBQWtELFNBQVMsVUFBVTtTQUNuRixDQUFDLENBQUM7UUFDSCxJQUFJLEdBQUcsQ0FBQyxTQUFTLENBQUMsSUFBSSxFQUFFLG9CQUFvQixFQUFFO1lBQzVDLEtBQUssRUFBRSxHQUFHLENBQUMsZ0JBQWdCLENBQUMsTUFBTSxFQUFFLGNBQWMsRUFBRSxHQUFHLENBQUM7WUFDeEQsV0FBVyxFQUFFLHFDQUFxQyxTQUFTLG1CQUFtQjtTQUMvRSxDQUFDLENBQUM7SUFDTCxDQUFDO0NBQ0Y7QUFoVUQsMENBZ1VDIiwic291cmNlc0NvbnRlbnQiOlsiaW1wb3J0ICogYXMgY2RrIGZyb20gJ2F3cy1jZGstbGliJztcbmltcG9ydCAqIGFzIGxhbWJkYSBmcm9tICdhd3MtY2RrLWxpYi9hd3MtbGFtYmRhJztcbmltcG9ydCAqIGFzIGFwaWdhdGV3YXkgZnJvbSAnYXdzLWNkay1saWIvYXdzLWFwaWdhdGV3YXknO1xuaW1wb3J0ICogYXMgbG9ncyBmcm9tICdhd3MtY2RrLWxpYi9hd3MtbG9ncyc7XG5pbXBvcnQgKiBhcyBzMyBmcm9tICdhd3MtY2RrLWxpYi9hd3MtczMnO1xuaW1wb3J0ICogYXMgczNhc3NldHMgZnJvbSAnYXdzLWNkay1saWIvYXdzLXMzLWFzc2V0cyc7XG5pbXBvcnQgKiBhcyBjb2RlYnVpbGQgZnJvbSAnYXdzLWNkay1saWIvYXdzLWNvZGVidWlsZCc7XG5pbXBvcnQgKiBhcyBjbG91ZHdhdGNoIGZyb20gJ2F3cy1jZGstbGliL2F3cy1jbG91ZHdhdGNoJztcbmltcG9ydCAqIGFzIGV2ZW50cyBmcm9tICdhd3MtY2RrLWxpYi9hd3MtZXZlbnRzJztcbmltcG9ydCAqIGFzIHRhcmdldHMgZnJvbSAnYXdzLWNkay1saWIvYXdzLWV2ZW50cy10YXJnZXRzJztcbmltcG9ydCAqIGFzIGttcyBmcm9tICdhd3MtY2RrLWxpYi9hd3Mta21zJztcbmltcG9ydCAqIGFzIGlhbSBmcm9tICdhd3MtY2RrLWxpYi9hd3MtaWFtJztcbmltcG9ydCAqIGFzIHNlY3JldHNtYW5hZ2VyIGZyb20gJ2F3cy1jZGstbGliL2F3cy1zZWNyZXRzbWFuYWdlcic7XG5pbXBvcnQgKiBhcyBwYXRoIGZyb20gJ3BhdGgnO1xuaW1wb3J0IHsgQ29uc3RydWN0IH0gZnJvbSAnY29uc3RydWN0cyc7XG5cbmludGVyZmFjZSBHb2RkYXJTdGFja1Byb3BzIGV4dGVuZHMgY2RrLlN0YWNrUHJvcHMge1xuICBzdGFnZTogJ2RldicgfCAncHJvZCc7XG59XG5cbmV4cG9ydCBjbGFzcyBSdXN0TGFtYmRhU3RhY2sgZXh0ZW5kcyBjZGsuU3RhY2sge1xuICBjb25zdHJ1Y3RvcihzY29wZTogQ29uc3RydWN0LCBpZDogc3RyaW5nLCBwcm9wczogR29kZGFyU3RhY2tQcm9wcykge1xuICAgIHN1cGVyKHNjb3BlLCBpZCwgcHJvcHMpO1xuXG4gICAgY29uc3QgeyBzdGFnZSB9ID0gcHJvcHM7XG4gICAgY29uc3Qgc3RhZ2VOYW1lID0gc3RhZ2UudG9VcHBlckNhc2UoKTtcblxuICAgIC8vIFMzIGJ1Y2tldCBmb3IgcHJvZHVjdCBpbWFnZSB1cGxvYWRzXG4gICAgY29uc3QgdXBsb2Fkc0J1Y2tldCA9IG5ldyBzMy5CdWNrZXQodGhpcywgYEdvZGRhcmQke3N0YWdlTmFtZX1VcGxvYWRzQnVja2V0YCwge1xuICAgICAgYnVja2V0TmFtZTogYGdvZGRhcmQtdXBsb2Fkcy0ke3N0YWdlfWAsXG4gICAgICBwdWJsaWNSZWFkQWNjZXNzOiB0cnVlLFxuICAgICAgYmxvY2tQdWJsaWNBY2Nlc3M6IHMzLkJsb2NrUHVibGljQWNjZXNzLkJMT0NLX0FDTFMsXG4gICAgICBjb3JzOiBbXG4gICAgICAgIHtcbiAgICAgICAgICBhbGxvd2VkTWV0aG9kczogW3MzLkh0dHBNZXRob2RzLkdFVCwgczMuSHR0cE1ldGhvZHMuUFVUXSxcbiAgICAgICAgICBhbGxvd2VkT3JpZ2luczogWycqJ10sXG4gICAgICAgICAgYWxsb3dlZEhlYWRlcnM6IFsnKiddLFxuICAgICAgICB9LFxuICAgICAgXSxcbiAgICAgIHJlbW92YWxQb2xpY3k6IGNkay5SZW1vdmFsUG9saWN5LlJFVEFJTixcbiAgICAgIHZlcnNpb25lZDogdHJ1ZSxcbiAgICB9KTtcblxuICAgIC8vIExhbWJkYSBmdW5jdGlvbiBmb3IgUnVzdCBjb2RlXG4gICAgLy8gVXNpbmcgQVJNNjQgYXJjaGl0ZWN0dXJlIGZvciB1cCB0byAzNCUgYmV0dGVyIHByaWNlIHBlcmZvcm1hbmNlIGFuZCAxOSUgYmV0dGVyIHBlcmZvcm1hbmNlXG4gICAgLy8gU2VlOiBodHRwczovL2F3cy5hbWF6b24uY29tL2Jsb2dzL2NvbXB1dGUvbWlncmF0aW5nLWF3cy1sYW1iZGEtZnVuY3Rpb25zLXRvLWFybS1iYXNlZC1hd3MtZ3Jhdml0b24yLXByb2Nlc3NvcnMvXG4gICAgY29uc3QgcnVzdExhbWJkYSA9IG5ldyBsYW1iZGEuRnVuY3Rpb24odGhpcywgYEdvZGRhcmQke3N0YWdlTmFtZX1MYW1iZGFgLCB7XG4gICAgICBmdW5jdGlvbk5hbWU6IGBnb2RkYXJkLSR7c3RhZ2V9YCxcbiAgICAgIHJ1bnRpbWU6IGxhbWJkYS5SdW50aW1lLlBST1ZJREVEX0FMMjAyMywgLy8gQW1hem9uIExpbnV4IDIwMjMgc3VwcG9ydHMgQVJNNjRcbiAgICAgIGFyY2hpdGVjdHVyZTogbGFtYmRhLkFyY2hpdGVjdHVyZS5BUk1fNjQsIC8vIEFXUyBHcmF2aXRvbjIgcHJvY2Vzc29yIChBUk02NClcbiAgICAgIGhhbmRsZXI6ICdib290c3RyYXAnLFxuICAgICAgY29kZTogbGFtYmRhLkNvZGUuZnJvbUFzc2V0KHBhdGguam9pbihfX2Rpcm5hbWUsICcuLi8uLi9sYW1iZGEvZ29kZGFyZC90YXJnZXQvbGFtYmRhL2dvZGRhcmQtYmFja2VuZCcpLCB7XG4gICAgICAgIGV4Y2x1ZGU6IFsnKionLCAnIWJvb3RzdHJhcCddLFxuICAgICAgfSksXG4gICAgICBtZW1vcnlTaXplOiBzdGFnZSA9PT0gJ2RldicgPyAxMjggOiAyNTYsXG4gICAgICB0aW1lb3V0OiBjZGsuRHVyYXRpb24uc2Vjb25kcygzMCksXG4gICAgICBlbnZpcm9ubWVudDoge1xuICAgICAgICBSVVNUX0xPRzogJ2luZm8nLFxuICAgICAgICBTM19VUExPQURfQlVDS0VUOiB1cGxvYWRzQnVja2V0LmJ1Y2tldE5hbWUsXG4gICAgICAgIFMzX0JBU0VfVVJMOiBgaHR0cHM6Ly8ke3VwbG9hZHNCdWNrZXQuYnVja2V0UmVnaW9uYWxEb21haW5OYW1lfWAsXG4gICAgICB9LFxuICAgICAgbG9nR3JvdXA6IG5ldyBsb2dzLkxvZ0dyb3VwKHRoaXMsIGBHb2RkYXJkJHtzdGFnZU5hbWV9TGFtYmRhTG9nR3JvdXBgLCB7XG4gICAgICAgIGxvZ0dyb3VwTmFtZTogYC9hd3MvbGFtYmRhL2dvZGRhcmQtJHtzdGFnZX1gLFxuICAgICAgICByZXRlbnRpb246IGxvZ3MuUmV0ZW50aW9uRGF5cy5PTkVfV0VFSyxcbiAgICAgICAgcmVtb3ZhbFBvbGljeTogY2RrLlJlbW92YWxQb2xpY3kuREVTVFJPWSxcbiAgICAgIH0pLFxuICAgICAgZGVzY3JpcHRpb246IGBHb2RkYXJkICR7c3RhZ2VOYW1lfSAtIEJhY2tlbmQgTGFtYmRhIGZ1bmN0aW9uIHdpdGggQVBJIGVuZHBvaW50c2AsXG4gICAgfSk7XG5cbiAgICAvLyBHcmFudCBMYW1iZGEgd3JpdGUgYWNjZXNzIHRvIHRoZSB1cGxvYWRzIGJ1Y2tldFxuICAgIHVwbG9hZHNCdWNrZXQuZ3JhbnRQdXQocnVzdExhbWJkYSk7XG5cbiAgICAvLyBBIHNlcGFyYXRlLCBzY2hlZHVsZWQgd29ya2VyIGRyYWlucyB0aGUgZHVyYWJsZSBGQ00gb3V0Ym94LiBJdCBkb2VzIG5vdFxuICAgIC8vIHJlcGxhY2Ugb3IgZXhwb3NlIHRoZSBleGlzdGluZyBBUEkgTGFtYmRhLCBzbyBtb2JpbGUvQVBJIEdhdGV3YXkgY2xpZW50c1xuICAgIC8vIHJldGFpbiB0aGVpciBjdXJyZW50IGVuZHBvaW50IGFuZCBiZWhhdmlvci5cbiAgICBjb25zdCBub3RpZmljYXRpb25QdXNoV29ya2VyID0gbmV3IGxhbWJkYS5GdW5jdGlvbih0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfU5vdGlmaWNhdGlvblB1c2hXb3JrZXJgLCB7XG4gICAgICBmdW5jdGlvbk5hbWU6IGBnb2RkYXJkLSR7c3RhZ2V9LW5vdGlmaWNhdGlvbi1wdXNoLXdvcmtlcmAsXG4gICAgICBydW50aW1lOiBsYW1iZGEuUnVudGltZS5QUk9WSURFRF9BTDIwMjMsXG4gICAgICBhcmNoaXRlY3R1cmU6IGxhbWJkYS5BcmNoaXRlY3R1cmUuQVJNXzY0LFxuICAgICAgaGFuZGxlcjogJ2Jvb3RzdHJhcCcsXG4gICAgICBjb2RlOiBsYW1iZGEuQ29kZS5mcm9tQXNzZXQocGF0aC5qb2luKF9fZGlybmFtZSwgJy4uLy4uL2xhbWJkYS9nb2RkYXJkL3RhcmdldC9sYW1iZGEvbm90aWZpY2F0aW9uX3B1c2hfd29ya2VyJyksIHtcbiAgICAgICAgZXhjbHVkZTogWycqKicsICchYm9vdHN0cmFwJ10sXG4gICAgICB9KSxcbiAgICAgIG1lbW9yeVNpemU6IDI1NixcbiAgICAgIHRpbWVvdXQ6IGNkay5EdXJhdGlvbi5zZWNvbmRzKDYwKSxcbiAgICAgIGVudmlyb25tZW50OiB7IFJVU1RfTE9HOiAnaW5mbycgfSxcbiAgICAgIGxvZ0dyb3VwOiBuZXcgbG9ncy5Mb2dHcm91cCh0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfU5vdGlmaWNhdGlvblB1c2hXb3JrZXJMb2dHcm91cGAsIHtcbiAgICAgICAgbG9nR3JvdXBOYW1lOiBgL2F3cy9sYW1iZGEvZ29kZGFyZC0ke3N0YWdlfS1ub3RpZmljYXRpb24tcHVzaC13b3JrZXJgLFxuICAgICAgICByZXRlbnRpb246IGxvZ3MuUmV0ZW50aW9uRGF5cy5PTkVfV0VFSyxcbiAgICAgICAgcmVtb3ZhbFBvbGljeTogY2RrLlJlbW92YWxQb2xpY3kuREVTVFJPWSxcbiAgICAgIH0pLFxuICAgICAgZGVzY3JpcHRpb246IGBHb2RkYXJkICR7c3RhZ2VOYW1lfSAtIHJlbGlhYmxlIEZDTSBwdXNoIG91dGJveCB3b3JrZXJgLFxuICAgIH0pO1xuICAgIC8vIFdha2UgdGhlIHdvcmtlciBhZnRlciBhIGNvbW1pdHRlZCBvdXRib3ggaW5zZXJ0OyB0aGUgc2NoZWR1bGUgYmVsb3cgaXNcbiAgICAvLyByZXRhaW5lZCBhcyB0aGUgcmVsaWFibGUgcmV0cnkvcmVjb3ZlcnkgcGF0aC5cbiAgICBub3RpZmljYXRpb25QdXNoV29ya2VyLmdyYW50SW52b2tlKHJ1c3RMYW1iZGEpO1xuICAgIG5ldyBldmVudHMuUnVsZSh0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfU5vdGlmaWNhdGlvblB1c2hTY2hlZHVsZWAsIHtcbiAgICAgIGRlc2NyaXB0aW9uOiBgRHJhaW5zIEdvZGRhcmQgJHtzdGFnZU5hbWV9IEZDTSBvdXRib3ggb25jZSBwZXIgbWludXRlLmAsXG4gICAgICBzY2hlZHVsZTogZXZlbnRzLlNjaGVkdWxlLnJhdGUoY2RrLkR1cmF0aW9uLm1pbnV0ZXMoMSkpLFxuICAgICAgdGFyZ2V0czogW25ldyB0YXJnZXRzLkxhbWJkYUZ1bmN0aW9uKG5vdGlmaWNhdGlvblB1c2hXb3JrZXIpXSxcbiAgICB9KTtcblxuXG4gICAgLy8gQVBJIEdhdGV3YXlcbiAgICBjb25zdCBhcGkgPSBuZXcgYXBpZ2F0ZXdheS5SZXN0QXBpKHRoaXMsIGBHb2RkYXJkJHtzdGFnZU5hbWV9QXBpYCwge1xuICAgICAgcmVzdEFwaU5hbWU6IGBHb2RkYXJkICR7c3RhZ2VOYW1lfSBBUElgLFxuICAgICAgZGVzY3JpcHRpb246IGAke3N0YWdlTmFtZX0gQVBJIEdhdGV3YXkgZm9yIEdvZGRhcmQgQmFja2VuZCBMYW1iZGEgZnVuY3Rpb25gLFxuICAgICAgYmluYXJ5TWVkaWFUeXBlczogWycqLyonXSxcbiAgICAgIGRlcGxveU9wdGlvbnM6IHtcbiAgICAgICAgc3RhZ2VOYW1lOiBzdGFnZSxcbiAgICAgICAgdHJhY2luZ0VuYWJsZWQ6IHN0YWdlID09PSAncHJvZCcsXG4gICAgICAgIG1ldHJpY3NFbmFibGVkOiB0cnVlLFxuICAgICAgfSxcbiAgICAgIC8vIENPUlMgaXMgaGFuZGxlZCBlbnRpcmVseSBieSBMYW1iZGEgbWlkZGxld2FyZSAoY29ycy5ycykuXG4gICAgICAvLyBEbyBOT1QgdXNlIGRlZmF1bHRDb3JzUHJlZmxpZ2h0T3B0aW9ucyBoZXJlIOKAlCBpdCBjcmVhdGVzIGEgTU9DS1xuICAgICAgLy8gaW50ZWdyYXRpb24gZm9yIE9QVElPTlMgdGhhdCBjb25mbGljdHMgd2l0aCBiaW5hcnlNZWRpYVR5cGVzOiBbJyovKiddLFxuICAgICAgLy8gY2F1c2luZyBBUEkgR2F0ZXdheSB0byBjb3JydXB0L3N0cmlwIENPUlMgaGVhZGVycyBmcm9tIHByZWZsaWdodCByZXNwb25zZXMuXG4gICAgfSk7XG5cbiAgICAvLyBMYW1iZGEgaW50ZWdyYXRpb24gd2l0aCBwcm94eVxuICAgIGNvbnN0IGxhbWJkYUludGVncmF0aW9uID0gbmV3IGFwaWdhdGV3YXkuTGFtYmRhSW50ZWdyYXRpb24ocnVzdExhbWJkYSwge1xuICAgICAgcHJveHk6IHRydWUsXG4gICAgfSk7XG5cbiAgICAvLyBIYW5kbGUgcm9vdCBwYXRoXG4gICAgYXBpLnJvb3QuYWRkTWV0aG9kKCdBTlknLCBsYW1iZGFJbnRlZ3JhdGlvbik7XG4gICAgLy8gRXhwbGljaXQgT1BUSU9OUyBvbiByb290IOKAlCBBTlkgZG9lcyBOT1QgZm9yd2FyZCBPUFRJT05TIGluIFJFU1QgQVBJXG4gICAgYXBpLnJvb3QuYWRkTWV0aG9kKCdPUFRJT05TJywgbGFtYmRhSW50ZWdyYXRpb24pO1xuXG4gICAgLy8gQ3JlYXRlIHByb3h5IHJlc291cmNlIGZvciBhbGwgb3RoZXIgcGF0aHNcbiAgICBjb25zdCBwcm94eVJlc291cmNlID0gYXBpLnJvb3QuYWRkUmVzb3VyY2UoJ3twcm94eSt9Jyk7XG4gICAgcHJveHlSZXNvdXJjZS5hZGRNZXRob2QoJ0FOWScsIGxhbWJkYUludGVncmF0aW9uKTtcbiAgICAvLyBFeHBsaWNpdCBPUFRJT05TIG9uIHByb3h5IOKAlCBmb3J3YXJkZWQgdG8gTGFtYmRhIENPUlMgbWlkZGxld2FyZVxuICAgIHByb3h5UmVzb3VyY2UuYWRkTWV0aG9kKCdPUFRJT05TJywgbGFtYmRhSW50ZWdyYXRpb24pO1xuXG4gICAgdGhpcy5hZGRCYWNrdXBQaXBlbGluZShhcGksIHVwbG9hZHNCdWNrZXQsIHN0YWdlKTtcblxuICAgIC8vIEFkZCBDT1JTIGhlYWRlcnMgdG8gQVBJIEdhdGV3YXkncyBvd24gZXJyb3IgcmVzcG9uc2VzICg0WFgvNVhYKVxuICAgIC8vIHNvIGJyb3dzZXJzIGNhbiByZWFkIGVycm9yIGRldGFpbHMgaW5zdGVhZCBvZiBzaG93aW5nIG9wYXF1ZSBDT1JTIGVycm9yc1xuICAgIGFwaS5hZGRHYXRld2F5UmVzcG9uc2UoJ0RlZmF1bHQ0WFgnLCB7XG4gICAgICB0eXBlOiBhcGlnYXRld2F5LlJlc3BvbnNlVHlwZS5ERUZBVUxUXzRYWCxcbiAgICAgIHJlc3BvbnNlSGVhZGVyczoge1xuICAgICAgICAnbWV0aG9kLnJlc3BvbnNlLmhlYWRlci5BY2Nlc3MtQ29udHJvbC1BbGxvdy1PcmlnaW4nOiBcIicqJ1wiLFxuICAgICAgICAnbWV0aG9kLnJlc3BvbnNlLmhlYWRlci5BY2Nlc3MtQ29udHJvbC1BbGxvdy1IZWFkZXJzJzogXCInQ29udGVudC1UeXBlLEF1dGhvcml6YXRpb24seC1yZXF1ZXN0LWlkLHgtc2Nob29sLWlkLHgtYXBpLWtleSdcIixcbiAgICAgICAgJ21ldGhvZC5yZXNwb25zZS5oZWFkZXIuQWNjZXNzLUNvbnRyb2wtQWxsb3ctTWV0aG9kcyc6IFwiJ0dFVCxQT1NULFBVVCxERUxFVEUsT1BUSU9OUyxQQVRDSCdcIixcbiAgICAgIH0sXG4gICAgfSk7XG4gICAgYXBpLmFkZEdhdGV3YXlSZXNwb25zZSgnRGVmYXVsdDVYWCcsIHtcbiAgICAgIHR5cGU6IGFwaWdhdGV3YXkuUmVzcG9uc2VUeXBlLkRFRkFVTFRfNVhYLFxuICAgICAgcmVzcG9uc2VIZWFkZXJzOiB7XG4gICAgICAgICdtZXRob2QucmVzcG9uc2UuaGVhZGVyLkFjY2Vzcy1Db250cm9sLUFsbG93LU9yaWdpbic6IFwiJyonXCIsXG4gICAgICAgICdtZXRob2QucmVzcG9uc2UuaGVhZGVyLkFjY2Vzcy1Db250cm9sLUFsbG93LUhlYWRlcnMnOiBcIidDb250ZW50LVR5cGUsQXV0aG9yaXphdGlvbix4LXJlcXVlc3QtaWQseC1zY2hvb2wtaWQseC1hcGkta2V5J1wiLFxuICAgICAgICAnbWV0aG9kLnJlc3BvbnNlLmhlYWRlci5BY2Nlc3MtQ29udHJvbC1BbGxvdy1NZXRob2RzJzogXCInR0VULFBPU1QsUFVULERFTEVURSxPUFRJT05TLFBBVENIJ1wiLFxuICAgICAgfSxcbiAgICB9KTtcblxuICAgIC8vIE91dHB1dHNcbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnQXBpVXJsJywge1xuICAgICAgdmFsdWU6IGFwaS51cmwsXG4gICAgICBkZXNjcmlwdGlvbjogYCR7c3RhZ2VOYW1lfSBBUEkgR2F0ZXdheSBVUkxgLFxuICAgICAgZXhwb3J0TmFtZTogYEdvZGRhcmQke3N0YWdlTmFtZX1BcGlVcmxgLFxuICAgIH0pO1xuXG4gICAgbmV3IGNkay5DZm5PdXRwdXQodGhpcywgJ0xhbWJkYUZ1bmN0aW9uTmFtZScsIHtcbiAgICAgIHZhbHVlOiBydXN0TGFtYmRhLmZ1bmN0aW9uTmFtZSxcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IExhbWJkYSBGdW5jdGlvbiBOYW1lYCxcbiAgICAgIGV4cG9ydE5hbWU6IGBHb2RkYXJkJHtzdGFnZU5hbWV9TGFtYmRhRnVuY3Rpb25OYW1lYCxcbiAgICB9KTtcblxuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdMYW1iZGFGdW5jdGlvbkFybicsIHtcbiAgICAgIHZhbHVlOiBydXN0TGFtYmRhLmZ1bmN0aW9uQXJuLFxuICAgICAgZGVzY3JpcHRpb246IGAke3N0YWdlTmFtZX0gTGFtYmRhIEZ1bmN0aW9uIEFSTmAsXG4gICAgICBleHBvcnROYW1lOiBgR29kZGFyZCR7c3RhZ2VOYW1lfUxhbWJkYUZ1bmN0aW9uQXJuYCxcbiAgICB9KTtcblxuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdOb3RpZmljYXRpb25QdXNoV29ya2VyRnVuY3Rpb25OYW1lJywge1xuICAgICAgdmFsdWU6IG5vdGlmaWNhdGlvblB1c2hXb3JrZXIuZnVuY3Rpb25OYW1lLFxuICAgICAgZGVzY3JpcHRpb246IGAke3N0YWdlTmFtZX0gRkNNIG91dGJveCB3b3JrZXIgZnVuY3Rpb24gbmFtZWAsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnVXBsb2Fkc0J1Y2tldE5hbWUnLCB7XG4gICAgICB2YWx1ZTogdXBsb2Fkc0J1Y2tldC5idWNrZXROYW1lLFxuICAgICAgZGVzY3JpcHRpb246IGAke3N0YWdlTmFtZX0gUzMgVXBsb2FkcyBCdWNrZXQgTmFtZWAsXG4gICAgICBleHBvcnROYW1lOiBgR29kZGFyZCR7c3RhZ2VOYW1lfVVwbG9hZHNCdWNrZXROYW1lYCxcbiAgICB9KTtcblxuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdVcGxvYWRzQnVja2V0VXJsJywge1xuICAgICAgdmFsdWU6IGBodHRwczovLyR7dXBsb2Fkc0J1Y2tldC5idWNrZXRSZWdpb25hbERvbWFpbk5hbWV9YCxcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IFMzIFVwbG9hZHMgQnVja2V0IEJhc2UgVVJMYCxcbiAgICAgIGV4cG9ydE5hbWU6IGBHb2RkYXJkJHtzdGFnZU5hbWV9VXBsb2Fkc0J1Y2tldFVybGAsXG4gICAgfSk7XG4gIH1cblxuICAvKipcbiAgICogVGhlIGRhdGFiYXNlIGJhY2t1cCBpcyBkZWxpYmVyYXRlbHkgaXNvbGF0ZWQgZnJvbSB0aGUgQVBJIExhbWJkYS4gVGhlXG4gICAqIFN1cGFiYXNlIENMSSBzdGFydHMgcGdfZHVtcCBpbiBEb2NrZXIsIHdoaWNoIGlzIHN1cHBvcnRlZCBieSBwcml2aWxlZ2VkXG4gICAqIENvZGVCdWlsZCBidXQgbm90IGJ5IExhbWJkYS5cbiAgICovXG4gIHByaXZhdGUgYWRkQmFja3VwUGlwZWxpbmUoXG4gICAgYXBpOiBhcGlnYXRld2F5LlJlc3RBcGksXG4gICAgdXBsb2Fkc0J1Y2tldDogczMuSUJ1Y2tldCxcbiAgICBzdGFnZTogJ2RldicgfCAncHJvZCcsXG4gICk6IHZvaWQge1xuICAgIGNvbnN0IHN0YWdlTmFtZSA9IHN0YWdlLnRvVXBwZXJDYXNlKCk7XG4gICAgY29uc3Qgc3RhZ2VJZCA9IHN0YWdlID09PSAnZGV2JyA/ICdEZXYnIDogJ1Byb2QnO1xuICAgIGNvbnN0IHJldGVudGlvbkRheXMgPSBzdGFnZSA9PT0gJ3Byb2QnID8gMzY1IDogOTA7XG4gICAgY29uc3QgYmFja3VwS2V5ID0gbmV3IGttcy5LZXkodGhpcywgYCR7c3RhZ2VJZH1CYWNrdXBLZXlgLCB7XG4gICAgICBhbGlhczogYGFsaWFzL2dvZGRhcmQtJHtzdGFnZX0tYmFja3Vwc2AsXG4gICAgICBlbmFibGVLZXlSb3RhdGlvbjogdHJ1ZSxcbiAgICAgIHJlbW92YWxQb2xpY3k6IGNkay5SZW1vdmFsUG9saWN5LlJFVEFJTixcbiAgICB9KTtcblxuICAgIGNvbnN0IGJhY2t1cEJ1Y2tldCA9IG5ldyBzMy5CdWNrZXQodGhpcywgYCR7c3RhZ2VJZH1CYWNrdXBCdWNrZXRgLCB7XG4gICAgICBidWNrZXROYW1lOiBjZGsuRm4uc3ViKGBnb2RkYXJkLSR7c3RhZ2V9LWJhY2t1cHMtXFwke0FXUzo6QWNjb3VudElkfS1cXCR7QVdTOjpSZWdpb259YCksXG4gICAgICBlbmNyeXB0aW9uOiBzMy5CdWNrZXRFbmNyeXB0aW9uLktNUyxcbiAgICAgIGVuY3J5cHRpb25LZXk6IGJhY2t1cEtleSxcbiAgICAgIGJ1Y2tldEtleUVuYWJsZWQ6IHRydWUsXG4gICAgICBibG9ja1B1YmxpY0FjY2VzczogczMuQmxvY2tQdWJsaWNBY2Nlc3MuQkxPQ0tfQUxMLFxuICAgICAgZW5mb3JjZVNTTDogdHJ1ZSxcbiAgICAgIHZlcnNpb25lZDogdHJ1ZSxcbiAgICAgIHJlbW92YWxQb2xpY3k6IGNkay5SZW1vdmFsUG9saWN5LlJFVEFJTixcbiAgICAgIGxpZmVjeWNsZVJ1bGVzOiBbe1xuICAgICAgICBpZDogYGV4cGlyZS0ke3N0YWdlfS1yZWNvdmVyeS1wb2ludHMtYWZ0ZXItJHtyZXRlbnRpb25EYXlzfS1kYXlzYCxcbiAgICAgICAgZW5hYmxlZDogdHJ1ZSxcbiAgICAgICAgZXhwaXJhdGlvbjogY2RrLkR1cmF0aW9uLmRheXMocmV0ZW50aW9uRGF5cyksXG4gICAgICAgIG5vbmN1cnJlbnRWZXJzaW9uRXhwaXJhdGlvbjogY2RrLkR1cmF0aW9uLmRheXMoNyksXG4gICAgICB9XSxcbiAgICB9KTtcblxuICAgIC8vIENyZWF0ZSB0aGlzIHNlY3JldCBiZWZvcmUgZGVwbG95aW5nIGFuZCBzdG9yZSBhIEpTT04gdmFsdWUgd2l0aCB0aGVcbiAgICAvLyBgZGF0YWJhc2VfdXJsYCBrZXkuIEtlZXBpbmcgdGhlIHZhbHVlIG91dHNpZGUgQ2xvdWRGb3JtYXRpb24gcHJldmVudHNcbiAgICAvLyBkYXRhYmFzZSBjcmVkZW50aWFscyBmcm9tIGFwcGVhcmluZyBpbiB0ZW1wbGF0ZXMgb3IgYnVpbGQgbG9ncy5cbiAgICBjb25zdCBkYXRhYmFzZVNlY3JldCA9IHNlY3JldHNtYW5hZ2VyLlNlY3JldC5mcm9tU2VjcmV0TmFtZVYyKFxuICAgICAgdGhpcyxcbiAgICAgIGAke3N0YWdlSWR9U3VwYWJhc2VCYWNrdXBEYXRhYmFzZVNlY3JldGAsXG4gICAgICBgZ29kZGFyZC8ke3N0YWdlfS9zdXBhYmFzZS1iYWNrdXBgLFxuICAgICk7XG4gICAgY29uc3QgZGF0YWJhc2VTZWNyZXROYW1lID0gYGdvZGRhcmQvJHtzdGFnZX0vc3VwYWJhc2UtYmFja3VwYDtcbiAgICBjb25zdCBwcm9qZWN0UmVmID0gbmV3IGNkay5DZm5QYXJhbWV0ZXIodGhpcywgYCR7c3RhZ2VJZH1TdXBhYmFzZVByb2plY3RSZWZgLCB7XG4gICAgICB0eXBlOiAnU3RyaW5nJyxcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IFN1cGFiYXNlIHByb2plY3QgcmVmZXJlbmNlIHJlY29yZGVkIGluIGVhY2ggYmFja3VwIG1hbmlmZXN0LmAsXG4gICAgfSk7XG5cbiAgICBjb25zdCB3b3JrZXJTb3VyY2UgPSBuZXcgczNhc3NldHMuQXNzZXQodGhpcywgYCR7c3RhZ2VJZH1CYWNrdXBXb3JrZXJTb3VyY2VgLCB7XG4gICAgICBwYXRoOiBwYXRoLmpvaW4oX19kaXJuYW1lLCAnLi4vLi4vYmFja3VwL3dvcmtlcicpLFxuICAgIH0pO1xuICAgIGNvbnN0IGJhY2t1cFByb2plY3QgPSBuZXcgY29kZWJ1aWxkLlByb2plY3QodGhpcywgYCR7c3RhZ2VJZH1TdXBhYmFzZUJhY2t1cFByb2plY3RgLCB7XG4gICAgICBwcm9qZWN0TmFtZTogYGdvZGRhcmQtJHtzdGFnZX0tc3VwYWJhc2UtYmFja3VwYCxcbiAgICAgIGRlc2NyaXB0aW9uOiBgQ3JlYXRlcyBlbmNyeXB0ZWQgbG9naWNhbCBTdXBhYmFzZSAke3N0YWdlTmFtZX0gcmVjb3ZlcnkgYnVuZGxlcyBpbiBTMy5gLFxuICAgICAgc291cmNlOiBjb2RlYnVpbGQuU291cmNlLnMzKHtcbiAgICAgICAgYnVja2V0OiB3b3JrZXJTb3VyY2UuYnVja2V0LFxuICAgICAgICBwYXRoOiB3b3JrZXJTb3VyY2UuczNPYmplY3RLZXksXG4gICAgICB9KSxcbiAgICAgIGJ1aWxkU3BlYzogY29kZWJ1aWxkLkJ1aWxkU3BlYy5mcm9tU291cmNlRmlsZW5hbWUoJ2J1aWxkc3BlYy55bWwnKSxcbiAgICAgIGVudmlyb25tZW50OiB7XG4gICAgICAgIGJ1aWxkSW1hZ2U6IGNvZGVidWlsZC5MaW51eEJ1aWxkSW1hZ2UuU1RBTkRBUkRfN18wLFxuICAgICAgICBwcml2aWxlZ2VkOiB0cnVlLFxuICAgICAgICBjb21wdXRlVHlwZTogY29kZWJ1aWxkLkNvbXB1dGVUeXBlLk1FRElVTSxcbiAgICAgICAgZW52aXJvbm1lbnRWYXJpYWJsZXM6IHtcbiAgICAgICAgICBEQVRBQkFTRV9VUkw6IHtcbiAgICAgICAgICAgIHR5cGU6IGNvZGVidWlsZC5CdWlsZEVudmlyb25tZW50VmFyaWFibGVUeXBlLlNFQ1JFVFNfTUFOQUdFUixcbiAgICAgICAgICAgIC8vIEltcG9ydGVkIHNlY3JldHMgaGF2ZSBhIHBhcnRpYWwgQVJOIHdpdGhvdXQgU2VjcmV0cyBNYW5hZ2VyJ3NcbiAgICAgICAgICAgIC8vIHJhbmRvbSBzdWZmaXguIENvZGVCdWlsZCBtdXN0IHJlc29sdmUgdGhpcyBieSBzdGFibGUgbmFtZS5cbiAgICAgICAgICAgIHZhbHVlOiBgJHtkYXRhYmFzZVNlY3JldE5hbWV9OmRhdGFiYXNlX3VybGAsXG4gICAgICAgICAgfSxcbiAgICAgICAgICBCQUNLVVBfQlVDS0VUOiB7IHZhbHVlOiBiYWNrdXBCdWNrZXQuYnVja2V0TmFtZSB9LFxuICAgICAgICAgIFVQTE9BRFNfQlVDS0VUOiB7IHZhbHVlOiB1cGxvYWRzQnVja2V0LmJ1Y2tldE5hbWUgfSxcbiAgICAgICAgICBCQUNLVVBfRU5WSVJPTk1FTlQ6IHsgdmFsdWU6IHN0YWdlIH0sXG4gICAgICAgICAgU1VQQUJBU0VfUFJPSkVDVF9SRUY6IHsgdmFsdWU6IHByb2plY3RSZWYudmFsdWVBc1N0cmluZyB9LFxuICAgICAgICAgIFNVUEFCQVNFX0NMSV9WRVJTSU9OOiB7IHZhbHVlOiAnMi42Ny4xJyB9LFxuICAgICAgICB9LFxuICAgICAgfSxcbiAgICAgIHRpbWVvdXQ6IGNkay5EdXJhdGlvbi5ob3VycygyKSxcbiAgICAgIHF1ZXVlZFRpbWVvdXQ6IGNkay5EdXJhdGlvbi5taW51dGVzKDMwKSxcbiAgICAgIGNvbmN1cnJlbnRCdWlsZExpbWl0OiAxLFxuICAgICAgZW5jcnlwdGlvbktleTogYmFja3VwS2V5LFxuICAgICAgbG9nZ2luZzoge1xuICAgICAgICBjbG91ZFdhdGNoOiB7XG4gICAgICAgICAgbG9nR3JvdXA6IG5ldyBsb2dzLkxvZ0dyb3VwKHRoaXMsIGAke3N0YWdlSWR9U3VwYWJhc2VCYWNrdXBCdWlsZExvZ0dyb3VwYCwge1xuICAgICAgICAgICAgcmV0ZW50aW9uOiBsb2dzLlJldGVudGlvbkRheXMuT05FX01PTlRILFxuICAgICAgICAgICAgcmVtb3ZhbFBvbGljeTogY2RrLlJlbW92YWxQb2xpY3kuUkVUQUlOLFxuICAgICAgICAgIH0pLFxuICAgICAgICB9LFxuICAgICAgfSxcbiAgICB9KTtcbiAgICBkYXRhYmFzZVNlY3JldC5ncmFudFJlYWQoYmFja3VwUHJvamVjdCk7XG4gICAgd29ya2VyU291cmNlLmdyYW50UmVhZChiYWNrdXBQcm9qZWN0KTtcbiAgICBiYWNrdXBCdWNrZXQuZ3JhbnRSZWFkV3JpdGUoYmFja3VwUHJvamVjdCk7XG4gICAgdXBsb2Fkc0J1Y2tldC5ncmFudFJlYWQoYmFja3VwUHJvamVjdCk7XG5cbiAgICBjb25zdCBvcmNoZXN0cmF0b3IgPSBuZXcgbGFtYmRhLkZ1bmN0aW9uKHRoaXMsIGAke3N0YWdlSWR9QmFja3VwT3JjaGVzdHJhdG9yYCwge1xuICAgICAgZnVuY3Rpb25OYW1lOiBgZ29kZGFyZC0ke3N0YWdlfS1iYWNrdXAtb3JjaGVzdHJhdG9yYCxcbiAgICAgIHJ1bnRpbWU6IGxhbWJkYS5SdW50aW1lLlBZVEhPTl8zXzEyLFxuICAgICAgYXJjaGl0ZWN0dXJlOiBsYW1iZGEuQXJjaGl0ZWN0dXJlLkFSTV82NCxcbiAgICAgIGhhbmRsZXI6ICdhcHAuaGFuZGxlcicsXG4gICAgICBjb2RlOiBsYW1iZGEuQ29kZS5mcm9tQXNzZXQocGF0aC5qb2luKF9fZGlybmFtZSwgJy4uLy4uL2JhY2t1cC9vcmNoZXN0cmF0b3InKSksXG4gICAgICB0aW1lb3V0OiBjZGsuRHVyYXRpb24uc2Vjb25kcygzMCksXG4gICAgICBtZW1vcnlTaXplOiAyNTYsXG4gICAgICBlbnZpcm9ubWVudDogeyBCQUNLVVBfUFJPSkVDVF9OQU1FOiBiYWNrdXBQcm9qZWN0LnByb2plY3ROYW1lIH0sXG4gICAgICBsb2dHcm91cDogbmV3IGxvZ3MuTG9nR3JvdXAodGhpcywgYCR7c3RhZ2VJZH1CYWNrdXBPcmNoZXN0cmF0b3JMb2dHcm91cGAsIHtcbiAgICAgICAgcmV0ZW50aW9uOiBsb2dzLlJldGVudGlvbkRheXMuT05FX01PTlRILFxuICAgICAgICByZW1vdmFsUG9saWN5OiBjZGsuUmVtb3ZhbFBvbGljeS5SRVRBSU4sXG4gICAgICB9KSxcbiAgICB9KTtcbiAgICBvcmNoZXN0cmF0b3IuYWRkVG9Sb2xlUG9saWN5KG5ldyBpYW0uUG9saWN5U3RhdGVtZW50KHtcbiAgICAgIGFjdGlvbnM6IFsnY29kZWJ1aWxkOlN0YXJ0QnVpbGQnXSxcbiAgICAgIHJlc291cmNlczogW2JhY2t1cFByb2plY3QucHJvamVjdEFybl0sXG4gICAgfSkpO1xuXG4gICAgY29uc3Qgb3BzID0gYXBpLnJvb3QuYWRkUmVzb3VyY2UoJ29wcycpO1xuICAgIGNvbnN0IGJhY2t1cHMgPSBvcHMuYWRkUmVzb3VyY2UoJ2JhY2t1cHMnKTtcbiAgICBiYWNrdXBzLmFkZE1ldGhvZCgnUE9TVCcsIG5ldyBhcGlnYXRld2F5LkxhbWJkYUludGVncmF0aW9uKG9yY2hlc3RyYXRvciksIHtcbiAgICAgIGF1dGhvcml6YXRpb25UeXBlOiBhcGlnYXRld2F5LkF1dGhvcml6YXRpb25UeXBlLklBTSxcbiAgICB9KTtcblxuICAgIG5ldyBjbG91ZHdhdGNoLkFsYXJtKHRoaXMsIGAke3N0YWdlSWR9QmFja3VwQnVpbGRGYWlsdXJlQWxhcm1gLCB7XG4gICAgICBhbGFybURlc2NyaXB0aW9uOiBgQSAke3N0YWdlTmFtZX0gU3VwYWJhc2UgYmFja3VwIENvZGVCdWlsZCBqb2IgZmFpbGVkLmAsXG4gICAgICBtZXRyaWM6IGJhY2t1cFByb2plY3QubWV0cmljRmFpbGVkQnVpbGRzKHsgcGVyaW9kOiBjZGsuRHVyYXRpb24uZGF5cygxKSB9KSxcbiAgICAgIHRocmVzaG9sZDogMSxcbiAgICAgIGV2YWx1YXRpb25QZXJpb2RzOiAxLFxuICAgICAgdHJlYXRNaXNzaW5nRGF0YTogY2xvdWR3YXRjaC5UcmVhdE1pc3NpbmdEYXRhLk5PVF9CUkVBQ0hJTkcsXG4gICAgfSk7XG4gICAgbmV3IGNsb3Vkd2F0Y2guQWxhcm0odGhpcywgYCR7c3RhZ2VJZH1CYWNrdXBPcmNoZXN0cmF0b3JFcnJvckFsYXJtYCwge1xuICAgICAgYWxhcm1EZXNjcmlwdGlvbjogYFRoZSAke3N0YWdlTmFtZX0gU3VwYWJhc2UgYmFja3VwIG9yY2hlc3RyYXRvciBmYWlsZWQgdG8gc3RhcnQgYSBidWlsZC5gLFxuICAgICAgbWV0cmljOiBvcmNoZXN0cmF0b3IubWV0cmljRXJyb3JzKHsgcGVyaW9kOiBjZGsuRHVyYXRpb24uZGF5cygxKSB9KSxcbiAgICAgIHRocmVzaG9sZDogMSxcbiAgICAgIGV2YWx1YXRpb25QZXJpb2RzOiAxLFxuICAgICAgdHJlYXRNaXNzaW5nRGF0YTogY2xvdWR3YXRjaC5UcmVhdE1pc3NpbmdEYXRhLk5PVF9CUkVBQ0hJTkcsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnQmFja3VwQnVja2V0TmFtZScsIHsgdmFsdWU6IGJhY2t1cEJ1Y2tldC5idWNrZXROYW1lIH0pO1xuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdCYWNrdXBBcGlQYXRoJywge1xuICAgICAgdmFsdWU6IGAke2FwaS51cmx9b3BzL2JhY2t1cHNgLFxuICAgICAgZGVzY3JpcHRpb246IGBJQU0tYXV0aGVudGljYXRlZCBlbmRwb2ludCB0byBtYW51YWxseSBzdGFydCBhICR7c3RhZ2VOYW1lfSBiYWNrdXAuYCxcbiAgICB9KTtcbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnQmFja3VwQXBpSW52b2tlQXJuJywge1xuICAgICAgdmFsdWU6IGFwaS5hcm5Gb3JFeGVjdXRlQXBpKCdQT1NUJywgJy9vcHMvYmFja3VwcycsICcqJyksXG4gICAgICBkZXNjcmlwdGlvbjogYElBTSByZXNvdXJjZSBBUk4gZm9yIGludm9raW5nIHRoZSAke3N0YWdlTmFtZX0gYmFja3VwIGVuZHBvaW50LmAsXG4gICAgfSk7XG4gIH1cbn1cbiJdfQ==