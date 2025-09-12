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
        bundling: {
          image: lambda.Runtime.PROVIDED_AL2023.bundlingImage,
          command: [
            'bash', '-c',
            [
              'cd /asset-input',
              'cargo lambda build --release --arm64',
              'cp target/lambda/hello-world/bootstrap /asset-output/'
            ].join(' && ')
          ],
          environment: {
            CARGO_HOME: '/tmp/cargo-home',
            RUSTUP_HOME: '/tmp/rustup-home',
          },
          user: 'root',
        },
      }),
      memorySize: 256,
      timeout: cdk.Duration.seconds(30),
      environment: {
        RUST_LOG: 'info',
      },
      logRetention: logs.RetentionDays.ONE_WEEK,
      description: 'Rust Lambda function with Hello World API',
    });

    // API Gateway
    const api = new apigateway.RestApi(this, 'RustLambdaApi', {
      restApiName: 'Rust Lambda API',
      description: 'API Gateway for Rust Lambda function',
      deployOptions: {
        stageName: 'prod',
        tracingEnabled: true,
        loggingLevel: apigateway.MethodLoggingLevel.INFO,
        dataTraceEnabled: true,
        metricsEnabled: true,
      },
      defaultCorsPreflightOptions: {
        allowOrigins: apigateway.Cors.ALL_ORIGINS,
        allowMethods: apigateway.Cors.ALL_METHODS,
        allowHeaders: ['Content-Type', 'Authorization'],
      },
    });

    // Lambda integration
    const lambdaIntegration = new apigateway.LambdaIntegration(rustLambda, {
      requestTemplates: { 'application/json': '{ "statusCode": "200" }' },
    });

    // API routes
    api.root.addMethod('GET', lambdaIntegration);
    
    const helloResource = api.root.addResource('hello');
    const nameResource = helloResource.addResource('{name}');
    nameResource.addMethod('GET', lambdaIntegration);
    
    const healthResource = api.root.addResource('health');
    healthResource.addMethod('GET', lambdaIntegration);

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