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
  }
}
