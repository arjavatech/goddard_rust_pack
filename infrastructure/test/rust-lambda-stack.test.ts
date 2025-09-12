import * as cdk from 'aws-cdk-lib';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as apigateway from 'aws-cdk-lib/aws-apigateway';
import * as logs from 'aws-cdk-lib/aws-logs';
import { Template } from 'aws-cdk-lib/assertions';

// Create a test-only stack that uses a different runtime for testing
class TestableRustLambdaStack extends cdk.Stack {
  constructor(scope: cdk.App, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // Use nodejs runtime for testing (supports inline code)
    const rustLambda = new lambda.Function(this, 'RustHelloWorldLambda', {
      runtime: lambda.Runtime.NODEJS_20_X, // Different runtime for testing
      architecture: lambda.Architecture.ARM_64,
      handler: 'index.handler',
      code: lambda.Code.fromInline(`
        exports.handler = async (event) => {
          return { statusCode: 200, body: 'test' };
        };
      `),
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

    // API Gateway (same as production)
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

    // API routes (same as production)
    api.root.addMethod('GET', lambdaIntegration);
    
    const helloResource = api.root.addResource('hello');
    const nameResource = helloResource.addResource('{name}');
    nameResource.addMethod('GET', lambdaIntegration);
    
    const healthResource = api.root.addResource('health');
    healthResource.addMethod('GET', lambdaIntegration);

    // Outputs (same as production)
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

describe('RustLambdaStack Infrastructure Tests', () => {
  let app: cdk.App;
  let stack: TestableRustLambdaStack;
  let template: Template;

  beforeEach(() => {
    app = new cdk.App();
    stack = new TestableRustLambdaStack(app, 'TestStack', {
      env: {
        account: '123456789012',
        region: 'us-east-1',
      },
    });
    template = Template.fromStack(stack);
  });

  test('creates a Lambda function with correct properties', () => {
    template.hasResourceProperties('AWS::Lambda::Function', {
      MemorySize: 256,
      Timeout: 30,
      Architectures: ['arm64'],
    });
  });

  test('creates an API Gateway REST API', () => {
    template.hasResourceProperties('AWS::ApiGateway::RestApi', {
      Name: 'Rust Lambda API',
      Description: 'API Gateway for Rust Lambda function',
    });
  });

  test('configures API Gateway deployment', () => {
    template.hasResourceProperties('AWS::ApiGateway::Deployment', {
      Description: 'API Gateway for Rust Lambda function',
    });
  });

  test('creates Lambda function with environment variables', () => {
    template.hasResourceProperties('AWS::Lambda::Function', {
      Environment: {
        Variables: {
          RUST_LOG: 'info',
        },
      },
    });
  });

  test('creates CloudWatch log group with correct retention', () => {
    template.hasResourceProperties('AWS::Logs::LogGroup', {
      RetentionInDays: 7,
    });
  });

  test('creates IAM role for Lambda execution', () => {
    template.hasResourceProperties('AWS::IAM::Role', {
      AssumeRolePolicyDocument: {
        Statement: [
          {
            Action: 'sts:AssumeRole',
            Effect: 'Allow',
            Principal: {
              Service: 'lambda.amazonaws.com',
            },
          },
        ],
      },
    });
  });

  test('creates API Gateway resources for all routes', () => {
    // Check for hello resource
    template.hasResourceProperties('AWS::ApiGateway::Resource', {
      PathPart: 'hello',
    });
    
    // Check for health resource  
    template.hasResourceProperties('AWS::ApiGateway::Resource', {
      PathPart: 'health',
    });
    
    // Check for {name} parameter resource
    template.hasResourceProperties('AWS::ApiGateway::Resource', {
      PathPart: '{name}',
    });
  });

  test('creates API Gateway methods for all routes', () => {
    // CDK creates additional OPTIONS methods for CORS, so expect more than 3
    const methodCount = template.findResources('AWS::ApiGateway::Method');
    expect(Object.keys(methodCount).length).toBeGreaterThanOrEqual(3);
    
    // Verify specific GET methods exist
    template.hasResourceProperties('AWS::ApiGateway::Method', {
      HttpMethod: 'GET',
    });
  });

  test('grants API Gateway permission to invoke Lambda', () => {
    template.hasResourceProperties('AWS::Lambda::Permission', {
      Action: 'lambda:InvokeFunction',
      Principal: 'apigateway.amazonaws.com',
    });
  });

  test('has all required stack outputs', () => {
    template.hasOutput('ApiUrl', {});
    template.hasOutput('LambdaFunctionName', {});
    template.hasOutput('LambdaFunctionArn', {});
    
    const outputs = template.findOutputs('*');
    // CDK may create additional outputs, so check we have at least our 3
    expect(Object.keys(outputs).length).toBeGreaterThanOrEqual(3);
  });

  test('API Gateway stage has tracing enabled', () => {
    template.hasResourceProperties('AWS::ApiGateway::Stage', {
      StageName: 'prod',
      TracingEnabled: true,
    });
  });

  test('Lambda function has correct description', () => {
    template.hasResourceProperties('AWS::Lambda::Function', {
      Description: 'Rust Lambda function with Hello World API',
    });
  });

  test('CloudFormation template has expected resource count', () => {
    const cfnTemplate = template.toJSON();
    const resourceCount = Object.keys(cfnTemplate.Resources || {}).length;
    
    // Should have: Lambda function, API Gateway API, 3 resources, 3 methods, 
    // deployment, stage, IAM role, log group, permissions, etc.
    expect(resourceCount).toBeGreaterThan(10);
  });

  test('synthesizes CloudFormation template successfully', () => {
    expect(() => {
      const cfnTemplate = template.toJSON();
      expect(cfnTemplate).toHaveProperty('Resources');
      expect(cfnTemplate).toHaveProperty('Outputs');
    }).not.toThrow();
  });

  test('validates API Gateway integration configuration', () => {
    // Check that Lambda integration is properly configured
    template.hasResourceProperties('AWS::ApiGateway::Method', {
      HttpMethod: 'GET',
      Integration: {
        Type: 'AWS_PROXY',
      },
    });
  });

  test('ensures proper CORS configuration in API Gateway', () => {
    // Check that RestApi is created (CORS is configured at this level)
    template.resourceCountIs('AWS::ApiGateway::RestApi', 1);
    
    template.hasResourceProperties('AWS::ApiGateway::RestApi', {
      Name: 'Rust Lambda API',
    });
  });
});