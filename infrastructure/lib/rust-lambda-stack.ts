import * as cdk from 'aws-cdk-lib';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as apigateway from 'aws-cdk-lib/aws-apigateway';
import * as logs from 'aws-cdk-lib/aws-logs';
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
      },
      logGroup: new logs.LogGroup(this, `Goddard${stageName}LambdaLogGroup`, {
        logGroupName: `/aws/lambda/goddard-${stage}`,
        retention: logs.RetentionDays.ONE_WEEK,
        removalPolicy: cdk.RemovalPolicy.DESTROY,
      }),
      description: `Goddard ${stageName} - Backend Lambda function with API endpoints`,
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
      defaultCorsPreflightOptions: {
        allowOrigins: apigateway.Cors.ALL_ORIGINS,
        allowMethods: apigateway.Cors.ALL_METHODS,
        allowHeaders: ['Content-Type', 'Authorization', 'x-request-id', 'x-school-id', 'x-api-key'],
        exposeHeaders: ['Content-Disposition'],
      },
    });

    // Lambda integration with proxy
    const lambdaIntegration = new apigateway.LambdaIntegration(rustLambda, {
      proxy: true,
    });

    // Handle root path
    api.root.addMethod('ANY', lambdaIntegration);

    // Create proxy resource for all other paths
    const proxyResource = api.root.addResource('{proxy+}');
    proxyResource.addMethod('ANY', lambdaIntegration);

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
  }
}
