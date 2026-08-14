import * as cdk from 'aws-cdk-lib';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as apigateway from 'aws-cdk-lib/aws-apigateway';
import * as logs from 'aws-cdk-lib/aws-logs';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as s3assets from 'aws-cdk-lib/aws-s3-assets';
import * as codebuild from 'aws-cdk-lib/aws-codebuild';
import * as cloudwatch from 'aws-cdk-lib/aws-cloudwatch';
import * as kms from 'aws-cdk-lib/aws-kms';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import * as path from 'path';
import { Construct } from 'constructs';

interface GoddarStackProps extends cdk.StackProps {
  stage: 'dev' | 'prod';
}

export class RustLambdaStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: GoddarStackProps) {
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
  private addBackupPipeline(
    api: apigateway.RestApi,
    uploadsBucket: s3.IBucket,
    stage: 'dev' | 'prod',
  ): void {
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
    const databaseSecret = secretsmanager.Secret.fromSecretNameV2(
      this,
      `${stageId}SupabaseBackupDatabaseSecret`,
      `goddard/${stage}/supabase-backup`,
    );
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
