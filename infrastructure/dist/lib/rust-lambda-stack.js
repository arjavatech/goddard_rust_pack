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
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoicnVzdC1sYW1iZGEtc3RhY2suanMiLCJzb3VyY2VSb290IjoiIiwic291cmNlcyI6WyIuLi8uLi9saWIvcnVzdC1sYW1iZGEtc3RhY2sudHMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6Ijs7O0FBQUEsbUNBQW1DO0FBQ25DLGlEQUFpRDtBQUNqRCx5REFBeUQ7QUFDekQsNkNBQTZDO0FBQzdDLHlDQUF5QztBQUN6QyxzREFBc0Q7QUFDdEQsdURBQXVEO0FBQ3ZELHlEQUF5RDtBQUN6RCwyQ0FBMkM7QUFDM0MsMkNBQTJDO0FBQzNDLGlFQUFpRTtBQUNqRSw2QkFBNkI7QUFPN0IsTUFBYSxlQUFnQixTQUFRLEdBQUcsQ0FBQyxLQUFLO0lBQzVDLFlBQVksS0FBZ0IsRUFBRSxFQUFVLEVBQUUsS0FBdUI7UUFDL0QsS0FBSyxDQUFDLEtBQUssRUFBRSxFQUFFLEVBQUUsS0FBSyxDQUFDLENBQUM7UUFFeEIsTUFBTSxFQUFFLEtBQUssRUFBRSxHQUFHLEtBQUssQ0FBQztRQUN4QixNQUFNLFNBQVMsR0FBRyxLQUFLLENBQUMsV0FBVyxFQUFFLENBQUM7UUFFdEMsc0NBQXNDO1FBQ3RDLE1BQU0sYUFBYSxHQUFHLElBQUksRUFBRSxDQUFDLE1BQU0sQ0FBQyxJQUFJLEVBQUUsVUFBVSxTQUFTLGVBQWUsRUFBRTtZQUM1RSxVQUFVLEVBQUUsbUJBQW1CLEtBQUssRUFBRTtZQUN0QyxnQkFBZ0IsRUFBRSxJQUFJO1lBQ3RCLGlCQUFpQixFQUFFLEVBQUUsQ0FBQyxpQkFBaUIsQ0FBQyxVQUFVO1lBQ2xELElBQUksRUFBRTtnQkFDSjtvQkFDRSxjQUFjLEVBQUUsQ0FBQyxFQUFFLENBQUMsV0FBVyxDQUFDLEdBQUcsRUFBRSxFQUFFLENBQUMsV0FBVyxDQUFDLEdBQUcsQ0FBQztvQkFDeEQsY0FBYyxFQUFFLENBQUMsR0FBRyxDQUFDO29CQUNyQixjQUFjLEVBQUUsQ0FBQyxHQUFHLENBQUM7aUJBQ3RCO2FBQ0Y7WUFDRCxhQUFhLEVBQUUsR0FBRyxDQUFDLGFBQWEsQ0FB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QUMsQ0FBQztJQUNMLENBQUM7Q0FDRjtBQTVSRCwwQ0E0UkMiLCJzb3VyY2VzQ29udGVudCI6WyJpbXBvcnQgKiBhcyBjZGsgZnJvbSAnYXdzLWNkay1saWInO1xuaW1wb3J0ICogYXMgbGFtYmRhIGZyb20gJ2F3cy1jZGstbGliL2F3cy1sYW1iZGEnO1xuaW1wb3J0ICogYXMgYXBpZ2F0ZXdheSBmcm9tICdhd3MtY2RrLWxpYi9hd3MtYXBpZ2F0ZXdheSc7XG5pbXBvcnQgKiBhcyBsb2dzIGZyb20gJ2F3cy1jZGstbGliL2F3cy1sb2dzJztcbmltcG9ydCAqIGFzIHMzIGZyb20gJ2F3cy1jZGstbGliL2F3cy1zMyc7XG5pbXBvcnQgKiBhcyBzM2Fzc2V0cyBmcm9tICdhd3MtY2RrLWxpYi9hd3MtczMtYXNzZXRzJztcbmltcG9ydCAqIGFzIGNvZGVidWlsZCBmcm9tICdhd3MtY2RrLWxpYi9hd3MtY29kZWJ1aWxkJztcbmltcG9ydCAqIGFzIGNsb3Vkd2F0Y2ggZnJvbSAnYXdzLWNkay1saWIvYXdzLWNsb3Vkd2F0Y2gnO1xuaW1wb3J0ICogYXMga21zIGZyb20gJ2F3cy1jZGstbGliL2F3cy1rbXMnO1xuaW1wb3J0ICogYXMgaWFtIGZyb20gJ2F3cy1jZGstbGliL2F3cy1pYW0nO1xuaW1wb3J0ICogYXMgc2VjcmV0c21hbmFnZXIgZnJvbSAnYXdzLWNkay1saWIvYXdzLXNlY3JldHNtYW5hZ2VyJztcbmltcG9ydCAqIGFzIHBhdGggZnJvbSAncGF0aCc7XG5pbXBvcnQgeyBDb25zdHJ1Y3QgfSBmcm9tICdjb25zdHJ1Y3RzJztcblxuaW50ZXJmYWNlIEdvZGRhclN0YWNrUHJvcHMgZXh0ZW5kcyBjZGsuU3RhY2tQcm9wcyB7XG4gIHN0YWdlOiAnZGV2JyB8ICdwcm9kJztcbn1cblxuZXhwb3J0IGNsYXNzIFJ1c3RMYW1iZGFTdGFjayBleHRlbmRzIGNkay5TdGFjayB7XG4gIGNvbnN0cnVjdG9yKHNjb3BlOiBDb25zdHJ1Y3QsIGlkOiBzdHJpbmcsIHByb3BzOiBHb2RkYXJTdGFja1Byb3BzKSB7XG4gICAgc3VwZXIoc2NvcGUsIGlkLCBwcm9wcyk7XG5cbiAgICBjb25zdCB7IHN0YWdlIH0gPSBwcm9wcztcbiAgICBjb25zdCBzdGFnZU5hbWUgPSBzdGFnZS50b1VwcGVyQ2FzZSgpO1xuXG4gICAgLy8gUzMgYnVja2V0IGZvciBwcm9kdWN0IGltYWdlIHVwbG9hZHNcbiAgICBjb25zdCB1cGxvYWRzQnVja2V0ID0gbmV3IHMzLkJ1Y2tldCh0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfVVwbG9hZHNCdWNrZXRgLCB7XG4gICAgICBidWNrZXROYW1lOiBgZ29kZGFyZC11cGxvYWRzLSR7c3RhZ2V9YCxcbiAgICAgIHB1YmxpY1JlYWRBY2Nlc3M6IHRydWUsXG4gICAgICBibG9ja1B1YmxpY0FjY2VzczogczMuQmxvY2tQdWJsaWNBY2Nlc3MuQkxPQ0tfQUNMUyxcbiAgICAgIGNvcnM6IFtcbiAgICAgICAge1xuICAgICAgICAgIGFsbG93ZWRNZXRob2RzOiBbczMuSHR0cE1ldGhvZHMuR0VULCBzMy5IdHRwTWV0aG9kcy5QVVRdLFxuICAgICAgICAgIGFsbG93ZWRPcmlnaW5zOiBbJyonXSxcbiAgICAgICAgICBhbGxvd2VkSGVhZGVyczogWycqJ10sXG4gICAgICAgIH0sXG4gICAgICBdLFxuICAgICAgcmVtb3ZhbFBvbGljeTogY2RrLlJlbW92YWxQb2xpY3kuUkVUQUlOLFxuICAgICAgdmVyc2lvbmVkOiB0cnVlLFxuICAgIH0pO1xuXG4gICAgLy8gTGFtYmRhIGZ1bmN0aW9uIGZvciBSdXN0IGNvZGVcbiAgICAvLyBVc2luZyBBUk02NCBhcmNoaXRlY3R1cmUgZm9yIHVwIHRvIDM0JSBiZXR0ZXIgcHJpY2UgcGVyZm9ybWFuY2UgYW5kIDE5JSBiZXR0ZXIgcGVyZm9ybWFuY2VcbiAgICAvLyBTZWU6IGh0dHBzOi8vYXdzLmFtYXpvbi5jb20vYmxvZ3MvY29tcHV0ZS9taWdyYXRpbmctYXdzLWxhbWJkYS1mdW5jdGlvbnMtdG8tYXJtLWJhc2VkLWF3cy1ncmF2aXRvbjItcHJvY2Vzc29ycy9cbiAgICBjb25zdCBydXN0TGFtYmRhID0gbmV3IGxhbWJkYS5GdW5jdGlvbih0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfUxhbWJkYWAsIHtcbiAgICAgIGZ1bmN0aW9uTmFtZTogYGdvZGRhcmQtJHtzdGFnZX1gLFxuICAgICAgcnVudGltZTogbGFtYmRhLlJ1bnRpbWUuUFJPVklERURfQUwyMDIzLCAvLyBBbWF6b24gTGludXggMjAyMyBzdXBwb3J0cyBBUk02NFxuICAgICAgYXJjaGl0ZWN0dXJlOiBsYW1iZGEuQXJjaGl0ZWN0dXJlLkFSTV82NCwgLy8gQVdTIEdyYXZpdG9uMiBwcm9jZXNzb3IgKEFSTTY0KVxuICAgICAgaGFuZGxlcjogJ2Jvb3RzdHJhcCcsXG4gICAgICBjb2RlOiBsYW1iZGEuQ29kZS5mcm9tQXNzZXQocGF0aC5qb2luKF9fZGlybmFtZSwgJy4uLy4uL2xhbWJkYS9nb2RkYXJkL3RhcmdldC9sYW1iZGEvZ29kZGFyZC1iYWNrZW5kJyksIHtcbiAgICAgICAgZXhjbHVkZTogWycqKicsICchYm9vdHN0cmFwJ10sXG4gICAgICB9KSxcbiAgICAgIG1lbW9yeVNpemU6IHN0YWdlID09PSAnZGV2JyA/IDEyOCA6IDI1NixcbiAgICAgIHRpbWVvdXQ6IGNkay5EdXJhdGlvbi5zZWNvbmRzKDMwKSxcbiAgICAgIGVudmlyb25tZW50OiB7XG4gICAgICAgIFJVU1RfTE9HOiAnaW5mbycsXG4gICAgICAgIFMzX1VQTE9BRF9CVUNLRVQ6IHVwbG9hZHNCdWNrZXQuYnVja2V0TmFtZSxcbiAgICAgICAgUzNfQkFTRV9VUkw6IGBodHRwczovLyR7dXBsb2Fkc0J1Y2tldC5idWNrZXRSZWdpb25hbERvbWFpbk5hbWV9YCxcbiAgICAgIH0sXG4gICAgICBsb2dHcm91cDogbmV3IGxvZ3MuTG9nR3JvdXAodGhpcywgYEdvZGRhcmQke3N0YWdlTmFtZX1MYW1iZGFMb2dHcm91cGAsIHtcbiAgICAgICAgbG9nR3JvdXBOYW1lOiBgL2F3cy9sYW1iZGEvZ29kZGFyZC0ke3N0YWdlfWAsXG4gICAgICAgIHJldGVudGlvbjogbG9ncy5SZXRlbnRpb25EYXlzLk9ORV9XRUVLLFxuICAgICAgICByZW1vdmFsUG9saWN5OiBjZGsuUmVtb3ZhbFBvbGljeS5ERVNUUk9ZLFxuICAgICAgfSksXG4gICAgICBkZXNjcmlwdGlvbjogYEdvZGRhcmQgJHtzdGFnZU5hbWV9IC0gQmFja2VuZCBMYW1iZGEgZnVuY3Rpb24gd2l0aCBBUEkgZW5kcG9pbnRzYCxcbiAgICB9KTtcblxuICAgIC8vIEdyYW50IExhbWJkYSB3cml0ZSBhY2Nlc3MgdG8gdGhlIHVwbG9hZHMgYnVja2V0XG4gICAgdXBsb2Fkc0J1Y2tldC5ncmFudFB1dChydXN0TGFtYmRhKTtcblxuICAgIC8vIEFQSSBHYXRld2F5XG4gICAgY29uc3QgYXBpID0gbmV3IGFwaWdhdGV3YXkuUmVzdEFwaSh0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfUFwaWAsIHtcbiAgICAgIHJlc3RBcGlOYW1lOiBgR29kZGFyZCAke3N0YWdlTmFtZX0gQVBJYCxcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IEFQSSBHYXRld2F5IGZvciBHb2RkYXJkIEJhY2tlbmQgTGFtYmRhIGZ1bmN0aW9uYCxcbiAgICAgIGJpbmFyeU1lZGlhVHlwZXM6IFsnKi8qJ10sXG4gICAgICBkZXBsb3lPcHRpb25zOiB7XG4gICAgICAgIHN0YWdlTmFtZTogc3RhZ2UsXG4gICAgICAgIHRyYWNpbmdFbmFibGVkOiBzdGFnZSA9PT0gJ3Byb2QnLFxuICAgICAgICBtZXRyaWNzRW5hYmxlZDogdHJ1ZSxcbiAgICAgIH0sXG4gICAgICAvLyBDT1JTIGlzIGhhbmRsZWQgZW50aXJlbHkgYnkgTGFtYmRhIG1pZGRsZXdhcmUgKGNvcnMucnMpLlxuICAgICAgLy8gRG8gTk9UIHVzZSBkZWZhdWx0Q29yc1ByZWZsaWdodE9wdGlvbnMgaGVyZSDigJQgaXQgY3JlYXRlcyBhIE1PQ0tcbiAgICAgIC8vIGludGVncmF0aW9uIGZvciBPUFRJT05TIHRoYXQgY29uZmxpY3RzIHdpdGggYmluYXJ5TWVkaWFUeXBlczogWycqLyonXSxcbiAgICAgIC8vIGNhdXNpbmcgQVBJIEdhdGV3YXkgdG8gY29ycnVwdC9zdHJpcCBDT1JTIGhlYWRlcnMgZnJvbSBwcmVmbGlnaHQgcmVzcG9uc2VzLlxuICAgIH0pO1xuXG4gICAgLy8gTGFtYmRhIGludGVncmF0aW9uIHdpdGggcHJveHlcbiAgICBjb25zdCBsYW1iZGFJbnRlZ3JhdGlvbiA9IG5ldyBhcGlnYXRld2F5LkxhbWJkYUludGVncmF0aW9uKHJ1c3RMYW1iZGEsIHtcbiAgICAgIHByb3h5OiB0cnVlLFxuICAgIH0pO1xuXG4gICAgLy8gSGFuZGxlIHJvb3QgcGF0aFxuICAgIGFwaS5yb290LmFkZE1ldGhvZCgnQU5ZJywgbGFtYmRhSW50ZWdyYXRpb24pO1xuICAgIC8vIEV4cGxpY2l0IE9QVElPTlMgb24gcm9vdCDigJQgQU5ZIGRvZXMgTk9UIGZvcndhcmQgT1BUSU9OUyBpbiBSRVNUIEFQSVxuICAgIGFwaS5yb290LmFkZE1ldGhvZCgnT1BUSU9OUycsIGxhbWJkYUludGVncmF0aW9uKTtcblxuICAgIC8vIENyZWF0ZSBwcm94eSByZXNvdXJjZSBmb3IgYWxsIG90aGVyIHBhdGhzXG4gICAgY29uc3QgcHJveHlSZXNvdXJjZSA9IGFwaS5yb290LmFkZFJlc291cmNlKCd7cHJveHkrfScpO1xuICAgIHByb3h5UmVzb3VyY2UuYWRkTWV0aG9kKCdBTlknLCBsYW1iZGFJbnRlZ3JhdGlvbik7XG4gICAgLy8gRXhwbGljaXQgT1BUSU9OUyBvbiBwcm94eSDigJQgZm9yd2FyZGVkIHRvIExhbWJkYSBDT1JTIG1pZGRsZXdhcmVcbiAgICBwcm94eVJlc291cmNlLmFkZE1ldGhvZCgnT1BUSU9OUycsIGxhbWJkYUludGVncmF0aW9uKTtcblxuICAgIHRoaXMuYWRkQmFja3VwUGlwZWxpbmUoYXBpLCB1cGxvYWRzQnVja2V0LCBzdGFnZSk7XG5cbiAgICAvLyBBZGQgQ09SUyBoZWFkZXJzIHRvIEFQSSBHYXRld2F5J3Mgb3duIGVycm9yIHJlc3BvbnNlcyAoNFhYLzVYWClcbiAgICAvLyBzbyBicm93c2VycyBjYW4gcmVhZCBlcnJvciBkZXRhaWxzIGluc3RlYWQgb2Ygc2hvd2luZyBvcGFxdWUgQ09SUyBlcnJvcnNcbiAgICBhcGkuYWRkR2F0ZXdheVJlc3BvbnNlKCdEZWZhdWx0NFhYJywge1xuICAgICAgdHlwZTogYXBpZ2F0ZXdheS5SZXNwb25zZVR5cGUuREVGQVVMVF80WFgsXG4gICAgICByZXNwb25zZUhlYWRlcnM6IHtcbiAgICAgICAgJ21ldGhvZC5yZXNwb25zZS5oZWFkZXIuQWNjZXNzLUNvbnRyb2wtQWxsb3ctT3JpZ2luJzogXCInKidcIixcbiAgICAgICAgJ21ldGhvZC5yZXNwb25zZS5oZWFkZXIuQWNjZXNzLUNvbnRyb2wtQWxsb3ctSGVhZGVycyc6IFwiJ0NvbnRlbnQtVHlwZSxBdXRob3JpemF0aW9uLHgtcmVxdWVzdC1pZCx4LXNjaG9vbC1pZCx4LWFwaS1rZXknXCIsXG4gICAgICAgICdtZXRob2QucmVzcG9uc2UuaGVhZGVyLkFjY2Vzcy1Db250cm9sLUFsbG93LU1ldGhvZHMnOiBcIidHRVQsUE9TVCxQVVQsREVMRVRFLE9QVElPTlMsUEFUQ0gnXCIsXG4gICAgICB9LFxuICAgIH0pO1xuICAgIGFwaS5hZGRHYXRld2F5UmVzcG9uc2UoJ0RlZmF1bHQ1WFgnLCB7XG4gICAgICB0eXBlOiBhcGlnYXRld2F5LlJlc3BvbnNlVHlwZS5ERUZBVUxUXzVYWCxcbiAgICAgIHJlc3BvbnNlSGVhZGVyczoge1xuICAgICAgICAnbWV0aG9kLnJlc3BvbnNlLmhlYWRlci5BY2Nlc3MtQ29udHJvbC1BbGxvdy1PcmlnaW4nOiBcIicqJ1wiLFxuICAgICAgICAnbWV0aG9kLnJlc3BvbnNlLmhlYWRlci5BY2Nlc3MtQ29udHJvbC1BbGxvdy1IZWFkZXJzJzogXCInQ29udGVudC1UeXBlLEF1dGhvcml6YXRpb24seC1yZXF1ZXN0LWlkLHgtc2Nob29sLWlkLHgtYXBpLWtleSdcIixcbiAgICAgICAgJ21ldGhvZC5yZXNwb25zZS5oZWFkZXIuQWNjZXNzLUNvbnRyb2wtQWxsb3ctTWV0aG9kcyc6IFwiJ0dFVCxQT1NULFBVVCxERUxFVEUsT1BUSU9OUyxQQVRDSCdcIixcbiAgICAgIH0sXG4gICAgfSk7XG5cbiAgICAvLyBPdXRwdXRzXG4gICAgbmV3IGNkay5DZm5PdXRwdXQodGhpcywgJ0FwaVVybCcsIHtcbiAgICAgIHZhbHVlOiBhcGkudXJsLFxuICAgICAgZGVzY3JpcHRpb246IGAke3N0YWdlTmFtZX0gQVBJIEdhdGV3YXkgVVJMYCxcbiAgICAgIGV4cG9ydE5hbWU6IGBHb2RkYXJkJHtzdGFnZU5hbWV9QXBpVXJsYCxcbiAgICB9KTtcblxuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdMYW1iZGFGdW5jdGlvbk5hbWUnLCB7XG4gICAgICB2YWx1ZTogcnVzdExhbWJkYS5mdW5jdGlvbk5hbWUsXG4gICAgICBkZXNjcmlwdGlvbjogYCR7c3RhZ2VOYW1lfSBMYW1iZGEgRnVuY3Rpb24gTmFtZWAsXG4gICAgICBleHBvcnROYW1lOiBgR29kZGFyZCR7c3RhZ2VOYW1lfUxhbWJkYUZ1bmN0aW9uTmFtZWAsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnTGFtYmRhRnVuY3Rpb25Bcm4nLCB7XG4gICAgICB2YWx1ZTogcnVzdExhbWJkYS5mdW5jdGlvbkFybixcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IExhbWJkYSBGdW5jdGlvbiBBUk5gLFxuICAgICAgZXhwb3J0TmFtZTogYEdvZGRhcmQke3N0YWdlTmFtZX1MYW1iZGFGdW5jdGlvbkFybmAsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnVXBsb2Fkc0J1Y2tldE5hbWUnLCB7XG4gICAgICB2YWx1ZTogdXBsb2Fkc0J1Y2tldC5idWNrZXROYW1lLFxuICAgICAgZGVzY3JpcHRpb246IGAke3N0YWdlTmFtZX0gUzMgVXBsb2FkcyBCdWNrZXQgTmFtZWAsXG4gICAgICBleHBvcnROYW1lOiBgR29kZGFyZCR7c3RhZ2VOYW1lfVVwbG9hZHNCdWNrZXROYW1lYCxcbiAgICB9KTtcblxuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdVcGxvYWRzQnVja2V0VXJsJywge1xuICAgICAgdmFsdWU6IGBodHRwczovLyR7dXBsb2Fkc0J1Y2tldC5idWNrZXRSZWdpb25hbERvbWFpbk5hbWV9YCxcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IFMzIFVwbG9hZHMgQnVja2V0IEJhc2UgVVJMYCxcbiAgICAgIGV4cG9ydE5hbWU6IGBHb2RkYXJkJHtzdGFnZU5hbWV9VXBsb2Fkc0J1Y2tldFVybGAsXG4gICAgfSk7XG4gIH1cblxuICAvKipcbiAgICogVGhlIGRhdGFiYXNlIGJhY2t1cCBpcyBkZWxpYmVyYXRlbHkgaXNvbGF0ZWQgZnJvbSB0aGUgQVBJIExhbWJkYS4gVGhlXG4gICAqIFN1cGFiYXNlIENMSSBzdGFydHMgcGdfZHVtcCBpbiBEb2NrZXIsIHdoaWNoIGlzIHN1cHBvcnRlZCBieSBwcml2aWxlZ2VkXG4gICAqIENvZGVCdWlsZCBidXQgbm90IGJ5IExhbWJkYS5cbiAgICovXG4gIHByaXZhdGUgYWRkQmFja3VwUGlwZWxpbmUoXG4gICAgYXBpOiBhcGlnYXRld2F5LlJlc3RBcGksXG4gICAgdXBsb2Fkc0J1Y2tldDogczMuSUJ1Y2tldCxcbiAgICBzdGFnZTogJ2RldicgfCAncHJvZCcsXG4gICk6IHZvaWQge1xuICAgIGNvbnN0IHN0YWdlTmFtZSA9IHN0YWdlLnRvVXBwZXJDYXNlKCk7XG4gICAgY29uc3Qgc3RhZ2VJZCA9IHN0YWdlID09PSAnZGV2JyA/ICdEZXYnIDogJ1Byb2QnO1xuICAgIGNvbnN0IHJldGVudGlvbkRheXMgPSBzdGFnZSA9PT0gJ3Byb2QnID8gMzY1IDogOTA7XG4gICAgY29uc3QgYmFja3VwS2V5ID0gbmV3IGttcy5LZXkodGhpcywgYCR7c3RhZ2VJZH1CYWNrdXBLZXlgLCB7XG4gICAgICBhbGlhczogYGFsaWFzL2dvZGRhcmQtJHtzdGFnZX0tYmFja3Vwc2AsXG4gICAgICBlbmFibGVLZXlSb3RhdGlvbjogdHJ1ZSxcbiAgICAgIHJlbW92YWxQb2xpY3k6IGNkay5SZW1vdmFsUG9saWN5LlJFVEFJTixcbiAgICB9KTtcblxuICAgIGNvbnN0IGJhY2t1cEJ1Y2tldCA9IG5ldyBzMy5CdWNrZXQodGhpcywgYCR7c3RhZ2VJZH1CYWNrdXBCdWNrZXRgLCB7XG4gICAgICBidWNrZXROYW1lOiBjZGsuRm4uc3ViKGBnb2RkYXJkLSR7c3RhZ2V9LWJhY2t1cHMtXFwke0FXUzo6QWNjb3VudElkfS1cXCR7QVdTOjpSZWdpb259YCksXG4gICAgICBlbmNyeXB0aW9uOiBzMy5CdWNrZXRFbmNyeXB0aW9uLktNUyxcbiAgICAgIGVuY3J5cHRpb25LZXk6IGJhY2t1cEtleSxcbiAgICAgIGJ1Y2tldEtleUVuYWJsZWQ6IHRydWUsXG4gICAgICBibG9ja1B1YmxpY0FjY2VzczogczMuQmxvY2tQdWJsaWNBY2Nlc3MuQkxPQ0tfQUxMLFxuICAgICAgZW5mb3JjZVNTTDogdHJ1ZSxcbiAgICAgIHZlcnNpb25lZDogdHJ1ZSxcbiAgICAgIHJlbW92YWxQb2xpY3k6IGNkay5SZW1vdmFsUG9saWN5LlJFVEFJTixcbiAgICAgIGxpZmVjeWNsZVJ1bGVzOiBbe1xuICAgICAgICBpZDogYGV4cGlyZS0ke3N0YWdlfS1yZWNvdmVyeS1wb2ludHMtYWZ0ZXItJHtyZXRlbnRpb25EYXlzfS1kYXlzYCxcbiAgICAgICAgZW5hYmxlZDogdHJ1ZSxcbiAgICAgICAgZXhwaXJhdGlvbjogY2RrLkR1cmF0aW9uLmRheXMocmV0ZW50aW9uRGF5cyksXG4gICAgICAgIG5vbmN1cnJlbnRWZXJzaW9uRXhwaXJhdGlvbjogY2RrLkR1cmF0aW9uLmRheXMoNyksXG4gICAgICB9XSxcbiAgICB9KTtcblxuICAgIC8vIENyZWF0ZSB0aGlzIHNlY3JldCBiZWZvcmUgZGVwbG95aW5nIGFuZCBzdG9yZSBhIEpTT04gdmFsdWUgd2l0aCB0aGVcbiAgICAvLyBgZGF0YWJhc2VfdXJsYCBrZXkuIEtlZXBpbmcgdGhlIHZhbHVlIG91dHNpZGUgQ2xvdWRGb3JtYXRpb24gcHJldmVudHNcbiAgICAvLyBkYXRhYmFzZSBjcmVkZW50aWFscyBmcm9tIGFwcGVhcmluZyBpbiB0ZW1wbGF0ZXMgb3IgYnVpbGQgbG9ncy5cbiAgICBjb25zdCBkYXRhYmFzZVNlY3JldCA9IHNlY3JldHNtYW5hZ2VyLlNlY3JldC5mcm9tU2VjcmV0TmFtZVYyKFxuICAgICAgdGhpcyxcbiAgICAgIGAke3N0YWdlSWR9U3VwYWJhc2VCYWNrdXBEYXRhYmFzZVNlY3JldGAsXG4gICAgICBgZ29kZGFyZC8ke3N0YWdlfS9zdXBhYmFzZS1iYWNrdXBgLFxuICAgICk7XG4gICAgY29uc3QgZGF0YWJhc2VTZWNyZXROYW1lID0gYGdvZGRhcmQvJHtzdGFnZX0vc3VwYWJhc2UtYmFja3VwYDtcbiAgICBjb25zdCBwcm9qZWN0UmVmID0gbmV3IGNkay5DZm5QYXJhbWV0ZXIodGhpcywgYCR7c3RhZ2VJZH1TdXBhYmFzZVByb2plY3RSZWZgLCB7XG4gICAgICB0eXBlOiAnU3RyaW5nJyxcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IFN1cGFiYXNlIHByb2plY3QgcmVmZXJlbmNlIHJlY29yZGVkIGluIGVhY2ggYmFja3VwIG1hbmlmZXN0LmAsXG4gICAgfSk7XG5cbiAgICBjb25zdCB3b3JrZXJTb3VyY2UgPSBuZXcgczNhc3NldHMuQXNzZXQodGhpcywgYCR7c3RhZ2VJZH1CYWNrdXBXb3JrZXJTb3VyY2VgLCB7XG4gICAgICBwYXRoOiBwYXRoLmpvaW4oX19kaXJuYW1lLCAnLi4vLi4vYmFja3VwL3dvcmtlcicpLFxuICAgIH0pO1xuICAgIGNvbnN0IGJhY2t1cFByb2plY3QgPSBuZXcgY29kZWJ1aWxkLlByb2plY3QodGhpcywgYCR7c3RhZ2VJZH1TdXBhYmFzZUJhY2t1cFByb2plY3RgLCB7XG4gICAgICBwcm9qZWN0TmFtZTogYGdvZGRhcmQtJHtzdGFnZX0tc3VwYWJhc2UtYmFja3VwYCxcbiAgICAgIGRlc2NyaXB0aW9uOiBgQ3JlYXRlcyBlbmNyeXB0ZWQgbG9naWNhbCBTdXBhYmFzZSAke3N0YWdlTmFtZX0gcmVjb3ZlcnkgYnVuZGxlcyBpbiBTMy5gLFxuICAgICAgc291cmNlOiBjb2RlYnVpbGQuU291cmNlLnMzKHtcbiAgICAgICAgYnVja2V0OiB3b3JrZXJTb3VyY2UuYnVja2V0LFxuICAgICAgICBwYXRoOiB3b3JrZXJTb3VyY2UuczNPYmplY3RLZXksXG4gICAgICB9KSxcbiAgICAgIGJ1aWxkU3BlYzogY29kZWJ1aWxkLkJ1aWxkU3BlYy5mcm9tU291cmNlRmlsZW5hbWUoJ2J1aWxkc3BlYy55bWwnKSxcbiAgICAgIGVudmlyb25tZW50OiB7XG4gICAgICAgIGJ1aWxkSW1hZ2U6IGNvZGVidWlsZC5MaW51eEJ1aWxkSW1hZ2UuU1RBTkRBUkRfN18wLFxuICAgICAgICBwcml2aWxlZ2VkOiB0cnVlLFxuICAgICAgICBjb21wdXRlVHlwZTogY29kZWJ1aWxkLkNvbXB1dGVUeXBlLk1FRElVTSxcbiAgICAgICAgZW52aXJvbm1lbnRWYXJpYWJsZXM6IHtcbiAgICAgICAgICBEQVRBQkFTRV9VUkw6IHtcbiAgICAgICAgICAgIHR5cGU6IGNvZGVidWlsZC5CdWlsZEVudmlyb25tZW50VmFyaWFibGVUeXBlLlNFQ1JFVFNfTUFOQUdFUixcbiAgICAgICAgICAgIC8vIEltcG9ydGVkIHNlY3JldHMgaGF2ZSBhIHBhcnRpYWwgQVJOIHdpdGhvdXQgU2VjcmV0cyBNYW5hZ2VyJ3NcbiAgICAgICAgICAgIC8vIHJhbmRvbSBzdWZmaXguIENvZGVCdWlsZCBtdXN0IHJlc29sdmUgdGhpcyBieSBzdGFibGUgbmFtZS5cbiAgICAgICAgICAgIHZhbHVlOiBgJHtkYXRhYmFzZVNlY3JldE5hbWV9OmRhdGFiYXNlX3VybGAsXG4gICAgICAgICAgfSxcbiAgICAgICAgICBCQUNLVVBfQlVDS0VUOiB7IHZhbHVlOiBiYWNrdXBCdWNrZXQuYnVja2V0TmFtZSB9LFxuICAgICAgICAgIFVQTE9BRFNfQlVDS0VUOiB7IHZhbHVlOiB1cGxvYWRzQnVja2V0LmJ1Y2tldE5hbWUgfSxcbiAgICAgICAgICBCQUNLVVBfRU5WSVJPTk1FTlQ6IHsgdmFsdWU6IHN0YWdlIH0sXG4gICAgICAgICAgU1VQQUJBU0VfUFJPSkVDVF9SRUY6IHsgdmFsdWU6IHByb2plY3RSZWYudmFsdWVBc1N0cmluZyB9LFxuICAgICAgICAgIFNVUEFCQVNFX0NMSV9WRVJTSU9OOiB7IHZhbHVlOiAnMi42Ny4xJyB9LFxuICAgICAgICB9LFxuICAgICAgfSxcbiAgICAgIHRpbWVvdXQ6IGNkay5EdXJhdGlvbi5ob3VycygyKSxcbiAgICAgIHF1ZXVlZFRpbWVvdXQ6IGNkay5EdXJhdGlvbi5taW51dGVzKDMwKSxcbiAgICAgIGNvbmN1cnJlbnRCdWlsZExpbWl0OiAxLFxuICAgICAgZW5jcnlwdGlvbktleTogYmFja3VwS2V5LFxuICAgICAgbG9nZ2luZzoge1xuICAgICAgICBjbG91ZFdhdGNoOiB7XG4gICAgICAgICAgbG9nR3JvdXA6IG5ldyBsb2dzLkxvZ0dyb3VwKHRoaXMsIGAke3N0YWdlSWR9U3VwYWJhc2VCYWNrdXBCdWlsZExvZ0dyb3VwYCwge1xuICAgICAgICAgICAgcmV0ZW50aW9uOiBsb2dzLlJldGVudGlvbkRheXMuT05FX01PTlRILFxuICAgICAgICAgICAgcmVtb3ZhbFBvbGljeTogY2RrLlJlbW92YWxQb2xpY3kuUkVUQUlOLFxuICAgICAgICAgIH0pLFxuICAgICAgICB9LFxuICAgICAgfSxcbiAgICB9KTtcbiAgICBkYXRhYmFzZVNlY3JldC5ncmFudFJlYWQoYmFja3VwUHJvamVjdCk7XG4gICAgd29ya2VyU291cmNlLmdyYW50UmVhZChiYWNrdXBQcm9qZWN0KTtcbiAgICBiYWNrdXBCdWNrZXQuZ3JhbnRSZWFkV3JpdGUoYmFja3VwUHJvamVjdCk7XG4gICAgdXBsb2Fkc0J1Y2tldC5ncmFudFJlYWQoYmFja3VwUHJvamVjdCk7XG5cbiAgICBjb25zdCBvcmNoZXN0cmF0b3IgPSBuZXcgbGFtYmRhLkZ1bmN0aW9uKHRoaXMsIGAke3N0YWdlSWR9QmFja3VwT3JjaGVzdHJhdG9yYCwge1xuICAgICAgZnVuY3Rpb25OYW1lOiBgZ29kZGFyZC0ke3N0YWdlfS1iYWNrdXAtb3JjaGVzdHJhdG9yYCxcbiAgICAgIHJ1bnRpbWU6IGxhbWJkYS5SdW50aW1lLlBZVEhPTl8zXzEyLFxuICAgICAgYXJjaGl0ZWN0dXJlOiBsYW1iZGEuQXJjaGl0ZWN0dXJlLkFSTV82NCxcbiAgICAgIGhhbmRsZXI6ICdhcHAuaGFuZGxlcicsXG4gICAgICBjb2RlOiBsYW1iZGEuQ29kZS5mcm9tQXNzZXQocGF0aC5qb2luKF9fZGlybmFtZSwgJy4uLy4uL2JhY2t1cC9vcmNoZXN0cmF0b3InKSksXG4gICAgICB0aW1lb3V0OiBjZGsuRHVyYXRpb24uc2Vjb25kcygzMCksXG4gICAgICBtZW1vcnlTaXplOiAyNTYsXG4gICAgICBlbnZpcm9ubWVudDogeyBCQUNLVVBfUFJPSkVDVF9OQU1FOiBiYWNrdXBQcm9qZWN0LnByb2plY3ROYW1lIH0sXG4gICAgICBsb2dHcm91cDogbmV3IGxvZ3MuTG9nR3JvdXAodGhpcywgYCR7c3RhZ2VJZH1CYWNrdXBPcmNoZXN0cmF0b3JMb2dHcm91cGAsIHtcbiAgICAgICAgcmV0ZW50aW9uOiBsb2dzLlJldGVudGlvbkRheXMuT05FX01PTlRILFxuICAgICAgICByZW1vdmFsUG9saWN5OiBjZGsuUmVtb3ZhbFBvbGljeS5SRVRBSU4sXG4gICAgICB9KSxcbiAgICB9KTtcbiAgICBvcmNoZXN0cmF0b3IuYWRkVG9Sb2xlUG9saWN5KG5ldyBpYW0uUG9saWN5U3RhdGVtZW50KHtcbiAgICAgIGFjdGlvbnM6IFsnY29kZWJ1aWxkOlN0YXJ0QnVpbGQnXSxcbiAgICAgIHJlc291cmNlczogW2JhY2t1cFByb2plY3QucHJvamVjdEFybl0sXG4gICAgfSkpO1xuXG4gICAgY29uc3Qgb3BzID0gYXBpLnJvb3QuYWRkUmVzb3VyY2UoJ29wcycpO1xuICAgIGNvbnN0IGJhY2t1cHMgPSBvcHMuYWRkUmVzb3VyY2UoJ2JhY2t1cHMnKTtcbiAgICBiYWNrdXBzLmFkZE1ldGhvZCgnUE9TVCcsIG5ldyBhcGlnYXRld2F5LkxhbWJkYUludGVncmF0aW9uKG9yY2hlc3RyYXRvciksIHtcbiAgICAgIGF1dGhvcml6YXRpb25UeXBlOiBhcGlnYXRld2F5LkF1dGhvcml6YXRpb25UeXBlLklBTSxcbiAgICB9KTtcblxuICAgIG5ldyBjbG91ZHdhdGNoLkFsYXJtKHRoaXMsIGAke3N0YWdlSWR9QmFja3VwQnVpbGRGYWlsdXJlQWxhcm1gLCB7XG4gICAgICBhbGFybURlc2NyaXB0aW9uOiBgQSAke3N0YWdlTmFtZX0gU3VwYWJhc2UgYmFja3VwIENvZGVCdWlsZCBqb2IgZmFpbGVkLmAsXG4gICAgICBtZXRyaWM6IGJhY2t1cFByb2plY3QubWV0cmljRmFpbGVkQnVpbGRzKHsgcGVyaW9kOiBjZGsuRHVyYXRpb24uZGF5cygxKSB9KSxcbiAgICAgIHRocmVzaG9sZDogMSxcbiAgICAgIGV2YWx1YXRpb25QZXJpb2RzOiAxLFxuICAgICAgdHJlYXRNaXNzaW5nRGF0YTogY2xvdWR3YXRjaC5UcmVhdE1pc3NpbmdEYXRhLk5PVF9CUkVBQ0hJTkcsXG4gICAgfSk7XG4gICAgbmV3IGNsb3Vkd2F0Y2guQWxhcm0odGhpcywgYCR7c3RhZ2VJZH1CYWNrdXBPcmNoZXN0cmF0b3JFcnJvckFsYXJtYCwge1xuICAgICAgYWxhcm1EZXNjcmlwdGlvbjogYFRoZSAke3N0YWdlTmFtZX0gU3VwYWJhc2UgYmFja3VwIG9yY2hlc3RyYXRvciBmYWlsZWQgdG8gc3RhcnQgYSBidWlsZC5gLFxuICAgICAgbWV0cmljOiBvcmNoZXN0cmF0b3IubWV0cmljRXJyb3JzKHsgcGVyaW9kOiBjZGsuRHVyYXRpb24uZGF5cygxKSB9KSxcbiAgICAgIHRocmVzaG9sZDogMSxcbiAgICAgIGV2YWx1YXRpb25QZXJpb2RzOiAxLFxuICAgICAgdHJlYXRNaXNzaW5nRGF0YTogY2xvdWR3YXRjaC5UcmVhdE1pc3NpbmdEYXRhLk5PVF9CUkVBQ0hJTkcsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnQmFja3VwQnVja2V0TmFtZScsIHsgdmFsdWU6IGJhY2t1cEJ1Y2tldC5idWNrZXROYW1lIH0pO1xuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdCYWNrdXBBcGlQYXRoJywge1xuICAgICAgdmFsdWU6IGAke2FwaS51cmx9b3BzL2JhY2t1cHNgLFxuICAgICAgZGVzY3JpcHRpb246IGBJQU0tYXV0aGVudGljYXRlZCBlbmRwb2ludCB0byBtYW51YWxseSBzdGFydCBhICR7c3RhZ2VOYW1lfSBiYWNrdXAuYCxcbiAgICB9KTtcbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnQmFja3VwQXBpSW52b2tlQXJuJywge1xuICAgICAgdmFsdWU6IGFwaS5hcm5Gb3JFeGVjdXRlQXBpKCdQT1NUJywgJy9vcHMvYmFja3VwcycsICcqJyksXG4gICAgICBkZXNjcmlwdGlvbjogYElBTSByZXNvdXJjZSBBUk4gZm9yIGludm9raW5nIHRoZSAke3N0YWdlTmFtZX0gYmFja3VwIGVuZHBvaW50LmAsXG4gICAgfSk7XG4gIH1cbn1cbiJdfQ==