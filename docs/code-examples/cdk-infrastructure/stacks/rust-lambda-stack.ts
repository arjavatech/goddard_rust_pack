import * as cdk from 'aws-cdk-lib';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as apigateway from 'aws-cdk-lib/aws-apigateway';
import * as logs from 'aws-cdk-lib/aws-logs';
import * as iam from 'aws-cdk-lib/aws-iam';
import { Construct } from 'constructs';
import { RustLambdaConstruct } from '../constructs/rust-lambda';

export class RustLambdaStack extends cdk.Stack {
  public readonly lambdaFunction: lambda.Function;
  public readonly api: apigateway.RestApi;

  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // Create the Rust Lambda function using our construct
    const rustLambda = new RustLambdaConstruct(this, 'RustLambdaApi', {
      functionName: 'rust-hello-world-api',
      description: 'Rust Lambda function with Hello World API endpoints',
      timeout: cdk.Duration.seconds(30),
      memorySize: 256,
      environment: {
        RUST_LOG: 'info',
        LOG_LEVEL: 'INFO',
      },
    });

    this.lambdaFunction = rustLambda.lambdaFunction;

    // Create API Gateway
    this.api = new apigateway.RestApi(this, 'RustLambdaApi', {
      restApiName: 'Rust Lambda API',
      description: 'API Gateway for Rust Lambda Hello World endpoints',
      defaultCorsPreflightOptions: {
        allowOrigins: apigateway.Cors.ALL_ORIGINS,
        allowMethods: apigateway.Cors.ALL_METHODS,
        allowHeaders: ['Content-Type', 'X-Amz-Date', 'Authorization', 'X-Api-Key'],
      },
      deployOptions: {
        stageName: 'prod',
        metricsEnabled: true,
        loggingLevel: apigateway.MethodLoggingLevel.INFO,
        dataTraceEnabled: true,
        throttlingBurstLimit: 100,
        throttlingRateLimit: 50,
      },
      cloudWatchRole: true,
    });

    // Create Lambda integration
    const lambdaIntegration = new apigateway.LambdaIntegration(this.lambdaFunction, {
      requestTemplates: { 'application/json': '{ "statusCode": "200" }' },
      proxy: true,
    });

    // Root endpoint (/)
    this.api.root.addMethod('GET', lambdaIntegration);
    this.api.root.addMethod('OPTIONS', lambdaIntegration);

    // Health endpoint (/health)
    const healthResource = this.api.root.addResource('health');
    healthResource.addMethod('GET', lambdaIntegration);
    healthResource.addMethod('OPTIONS', lambdaIntegration);

    // Hello with name endpoint (/hello/{name})
    const helloResource = this.api.root.addResource('hello');
    const helloNameResource = helloResource.addResource('{name}');
    helloNameResource.addMethod('GET', lambdaIntegration);
    helloNameResource.addMethod('OPTIONS', lambdaIntegration);

    // Create CloudWatch Log Group for API Gateway
    const apiLogGroup = new logs.LogGroup(this, 'ApiGatewayLogGroup', {
      logGroupName: `/aws/apigateway/${this.api.restApiName}`,
      retention: logs.RetentionDays.ONE_WEEK,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    // Output important values
    new cdk.CfnOutput(this, 'ApiUrl', {
      value: this.api.url,
      description: 'API Gateway URL',
      exportName: 'RustLambdaApiUrl',
    });

    new cdk.CfnOutput(this, 'LambdaFunctionName', {
      value: this.lambdaFunction.functionName,
      description: 'Lambda Function Name',
      exportName: 'RustLambdaFunctionName',
    });

    new cdk.CfnOutput(this, 'LambdaFunctionArn', {
      value: this.lambdaFunction.functionArn,
      description: 'Lambda Function ARN',
      exportName: 'RustLambdaFunctionArn',
    });

    new cdk.CfnOutput(this, 'ApiId', {
      value: this.api.restApiId,
      description: 'API Gateway REST API ID',
      exportName: 'RustLambdaApiId',
    });
  }
}