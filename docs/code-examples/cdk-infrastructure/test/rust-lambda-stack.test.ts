import * as cdk from 'aws-cdk-lib';
import { Template, Match } from 'aws-cdk-lib/assertions';
import { RustLambdaStack } from '../stacks/rust-lambda-stack';

describe('RustLambdaStack', () => {
  let app: cdk.App;
  let stack: RustLambdaStack;
  let template: Template;

  beforeEach(() => {
    app = new cdk.App();
    stack = new RustLambdaStack(app, 'TestRustLambdaStack', {
      env: {
        account: '123456789012',
        region: 'us-east-1',
      },
    });
    template = Template.fromStack(stack);
  });

  describe('Lambda Function', () => {
    test('creates Lambda function with correct configuration', () => {
      template.hasResourceProperties('AWS::Lambda::Function', {
        FunctionName: 'rust-hello-world-api',
        Runtime: 'provided.al2023',
        Handler: 'bootstrap',
        Architecture: 'x86_64',
        Timeout: 30,
        MemorySize: 256,
        Environment: {
          Variables: {
            RUST_LOG: 'info',
            LOG_LEVEL: 'INFO',
          },
        },
      });
    });

    test('Lambda function has proper IAM role', () => {
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
        ManagedPolicyArns: [
          {
            'Fn::Join': [
              '',
              [
                'arn:',
                { Ref: 'AWS::Partition' },
                ':iam::aws:policy/service-role/AWSLambdaBasicExecutionRole',
              ],
            ],
          },
        ],
      });
    });

    test('Lambda function has tracing enabled', () => {
      template.hasResourceProperties('AWS::Lambda::Function', {
        TracingConfig: {
          Mode: 'Active',
        },
      });
    });
  });

  describe('API Gateway', () => {
    test('creates REST API with correct configuration', () => {
      template.hasResourceProperties('AWS::ApiGateway::RestApi', {
        Name: 'Rust Lambda API',
        Description: 'API Gateway for Rust Lambda Hello World endpoints',
      });
    });

    test('API Gateway has CORS configuration', () => {
      template.hasResource('AWS::ApiGateway::Method', {
        Properties: Match.objectLike({
          HttpMethod: 'OPTIONS',
        }),
      });
    });

    test('creates deployment with proper stage configuration', () => {
      template.hasResourceProperties('AWS::ApiGateway::Deployment', {
        Description: Match.anyValue(),
      });

      template.hasResourceProperties('AWS::ApiGateway::Stage', {
        StageName: 'prod',
        MethodSettings: [
          {
            HttpMethod: '*',
            LoggingLevel: 'INFO',
            DataTraceEnabled: true,
            MetricsEnabled: true,
            ResourcePath: '/*',
            ThrottlingBurstLimit: 100,
            ThrottlingRateLimit: 50,
          },
        ],
      });
    });

    test('creates proper API Gateway methods', () => {
      // Root endpoint GET method
      template.hasResource('AWS::ApiGateway::Method', {
        Properties: {
          HttpMethod: 'GET',
          ResourceId: {
            'Fn::GetAtt': [Match.stringLikeRegexp('.*RestApi.*'), 'RootResourceId'],
          },
        },
      });

      // Root endpoint OPTIONS method
      template.hasResource('AWS::ApiGateway::Method', {
        Properties: {
          HttpMethod: 'OPTIONS',
          ResourceId: {
            'Fn::GetAtt': [Match.stringLikeRegexp('.*RestApi.*'), 'RootResourceId'],
          },
        },
      });
    });

    test('creates health endpoint resource and methods', () => {
      template.hasResource('AWS::ApiGateway::Resource', {
        Properties: {
          PathPart: 'health',
        },
      });
    });

    test('creates hello endpoint with parameter resource', () => {
      template.hasResource('AWS::ApiGateway::Resource', {
        Properties: {
          PathPart: 'hello',
        },
      });

      template.hasResource('AWS::ApiGateway::Resource', {
        Properties: {
          PathPart: '{name}',
        },
      });
    });
  });

  describe('CloudWatch Integration', () => {
    test('creates CloudWatch Log Group for Lambda', () => {
      template.hasResourceProperties('AWS::Logs::LogGroup', {
        LogGroupName: '/aws/lambda/rust-hello-world-api',
        RetentionInDays: 7,
      });
    });

    test('creates CloudWatch Log Group for API Gateway', () => {
      template.hasResource('AWS::Logs::LogGroup', {
        Properties: Match.objectLike({
          LogGroupName: Match.stringLikeRegexp('/aws/apigateway/.*'),
          RetentionInDays: 7,
        }),
      });
    });
  });

  describe('Permissions', () => {
    test('API Gateway has permission to invoke Lambda', () => {
      template.hasResourceProperties('AWS::Lambda::Permission', {
        Action: 'lambda:InvokeFunction',
        Principal: 'apigateway.amazonaws.com',
      });
    });
  });

  describe('Stack Outputs', () => {
    test('exports API URL', () => {
      template.hasOutput('ApiUrl', {
        Description: 'API Gateway URL',
        Export: {
          Name: 'RustLambdaApiUrl',
        },
      });
    });

    test('exports Lambda function name', () => {
      template.hasOutput('LambdaFunctionName', {
        Description: 'Lambda Function Name',
        Export: {
          Name: 'RustLambdaFunctionName',
        },
      });
    });

    test('exports Lambda function ARN', () => {
      template.hasOutput('LambdaFunctionArn', {
        Description: 'Lambda Function ARN',
        Export: {
          Name: 'RustLambdaFunctionArn',
        },
      });
    });

    test('exports API Gateway REST API ID', () => {
      template.hasOutput('ApiId', {
        Description: 'API Gateway REST API ID',
        Export: {
          Name: 'RustLambdaApiId',
        },
      });
    });
  });

  describe('Tags', () => {
    test('stack has proper tags', () => {
      const stackTags = stack.tags.tagValues();
      expect(stackTags).toHaveProperty('Project', 'RustLambdaApi');
      expect(stackTags).toHaveProperty('ManagedBy', 'CDK');
    });
  });

  describe('Resource Naming', () => {
    test('resources have consistent naming', () => {
      // Check that Lambda function has expected name
      template.hasResourceProperties('AWS::Lambda::Function', {
        FunctionName: 'rust-hello-world-api',
      });

      // Check that API has expected name
      template.hasResourceProperties('AWS::ApiGateway::RestApi', {
        Name: 'Rust Lambda API',
      });
    });
  });
});

describe('RustLambdaStack Integration', () => {
  test('stack synthesizes without errors', () => {
    const app = new cdk.App();
    const stack = new RustLambdaStack(app, 'TestStack');
    
    expect(() => {
      app.synth();
    }).not.toThrow();
  });

  test('stack has expected resource count', () => {
    const app = new cdk.App();
    const stack = new RustLambdaStack(app, 'TestStack');
    const template = Template.fromStack(stack);

    // Count key resources
    const resources = template.toJSON().Resources;
    const resourceTypes = Object.values(resources).map((r: any) => r.Type);

    // Should have at least these resource types
    expect(resourceTypes).toContain('AWS::Lambda::Function');
    expect(resourceTypes).toContain('AWS::ApiGateway::RestApi');
    expect(resourceTypes).toContain('AWS::ApiGateway::Deployment');
    expect(resourceTypes).toContain('AWS::IAM::Role');
    expect(resourceTypes).toContain('AWS::Logs::LogGroup');
  });
});

describe('Environment Configuration', () => {
  test('works with different AWS regions', () => {
    const app = new cdk.App();
    
    const regions = ['us-east-1', 'us-west-2', 'eu-west-1'];
    
    regions.forEach(region => {
      expect(() => {
        new RustLambdaStack(app, `TestStack-${region}`, {
          env: { region, account: '123456789012' },
        });
      }).not.toThrow();
    });
  });
});

describe('Error Conditions', () => {
  test('handles missing environment gracefully', () => {
    const app = new cdk.App();
    
    expect(() => {
      new RustLambdaStack(app, 'TestStackNoEnv');
    }).not.toThrow();
  });
});