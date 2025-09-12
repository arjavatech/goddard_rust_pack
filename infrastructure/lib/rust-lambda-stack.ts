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
    const rustLambda = new lambda.Function(this, 'RustHelloWorldLambda', {
      runtime: lambda.Runtime.PROVIDED_AL2023,
      architecture: lambda.Architecture.ARM_64,
      handler: 'bootstrap',
      code: lambda.Code.fromAsset(path.join(__dirname, '../../lambda/hello-world/target/lambda/hello-world'), {
        exclude: ['**', '!bootstrap'],
      }),
      memorySize: 256,
      timeout: cdk.Duration.seconds(30),
      environment: {
        RUST_LOG: 'info',
      },
      logGroup: new logs.LogGroup(this, 'RustLambdaLogGroup', {
        retention: logs.RetentionDays.ONE_WEEK,
        removalPolicy: cdk.RemovalPolicy.DESTROY,
      }),
      description: 'Rust Lambda function with Hello World API',
    });

    // API Gateway
    const api = new apigateway.RestApi(this, 'RustLambdaApi', {
      restApiName: 'Rust Lambda API',
      description: 'API Gateway for Rust Lambda function',
      deployOptions: {
        stageName: 'prod',
        tracingEnabled: true,
        metricsEnabled: true,
      },
      defaultCorsPreflightOptions: {
        allowOrigins: apigateway.Cors.ALL_ORIGINS,
        allowMethods: apigateway.Cors.ALL_METHODS,
        allowHeaders: ['Content-Type', 'Authorization'],
      },
    });

    // Lambda integration with proxy
    const lambdaIntegration = new apigateway.LambdaIntegration(rustLambda, {
      proxy: true,
    });

    // Create proxy resource that handles all paths and methods
    const proxyResource = api.root.addProxy({
      defaultIntegration: lambdaIntegration,
      anyMethod: true,
    });

    // Outputs
    new cdk.CfnOutput(this, 'ApiUrl', {
      value: api.url,
      description: 'API Gateway URL',
      exportName: 'RustLambdaApiUrl',
    });

    new cdk.CfnOutput(this, 'LambdaFunctionName', {
      value: rustLambda.functionName,
      description: 'Lambda Function Name',
      exportName: 'RustLambdaFunctionName',
    });

    new cdk.CfnOutput(this, 'LambdaFunctionArn', {
      value: rustLambda.functionArn,
      description: 'Lambda Function ARN',
      exportName: 'RustLambdaFunctionArn',
    });
  }
}