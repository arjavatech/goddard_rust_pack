import * as cdk from 'aws-cdk-lib';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as apigateway from 'aws-cdk-lib/aws-apigateway';
import * as logs from 'aws-cdk-lib/aws-logs';
import * as path from 'path';
import { Construct } from 'constructs';

export class RustLambdaStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // Lambda function for Rust code
    // Using ARM64 architecture for up to 34% better price performance and 19% better performance
    // See: https://aws.amazon.com/blogs/compute/migrating-aws-lambda-functions-to-arm-based-aws-graviton2-processors/
    const rustLambda = new lambda.Function(this, 'GoddardProdLambda', {
      functionName: 'goddard-prod',
      runtime: lambda.Runtime.PROVIDED_AL2023, // Amazon Linux 2023 supports ARM64
      architecture: lambda.Architecture.ARM_64, // AWS Graviton2 processor (ARM64)
      handler: 'bootstrap',
      code: lambda.Code.fromAsset(path.join(__dirname, '../../lambda/goddard/target/lambda/goddard-backend'), {
        exclude: ['**', '!bootstrap'],
      }),
      memorySize: 256,
      timeout: cdk.Duration.seconds(30),
      environment: {
        RUST_LOG: 'info',
      },
      logGroup: new logs.LogGroup(this, 'GoddardProdLambdaLogGroup', {
        logGroupName: '/aws/lambda/goddard-prod',
        retention: logs.RetentionDays.ONE_WEEK,
        removalPolicy: cdk.RemovalPolicy.DESTROY,
      }),
      description: 'Goddard Production - Backend Lambda function with API endpoints',
    });

    // API Gateway
    const api = new apigateway.RestApi(this, 'GoddardProdApi', {
      restApiName: 'Goddard Production API',
      description: 'Production API Gateway for Goddard Backend Lambda function',
      deployOptions: {
        stageName: 'prod',
        tracingEnabled: true,
        metricsEnabled: true,
      },
      defaultCorsPreflightOptions: {
        allowOrigins: apigateway.Cors.ALL_ORIGINS,
        allowMethods: apigateway.Cors.ALL_METHODS,
        allowHeaders: ['Content-Type', 'Authorization', 'x-request-id', 'x-school-id', 'x-api-key'],
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
      description: 'Production API Gateway URL',
      exportName: 'GoddardProdApiUrl',
    });

    new cdk.CfnOutput(this, 'LambdaFunctionName', {
      value: rustLambda.functionName,
      description: 'Production Lambda Function Name',
      exportName: 'GoddardProdLambdaFunctionName',
    });

    new cdk.CfnOutput(this, 'LambdaFunctionArn', {
      value: rustLambda.functionArn,
      description: 'Production Lambda Function ARN',
      exportName: 'GoddardProdLambdaFunctionArn',
    });
  }
}