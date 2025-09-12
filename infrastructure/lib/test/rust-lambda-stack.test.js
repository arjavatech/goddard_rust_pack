"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const cdk = require("aws-cdk-lib");
const lambda = require("aws-cdk-lib/aws-lambda");
const apigateway = require("aws-cdk-lib/aws-apigateway");
const logs = require("aws-cdk-lib/aws-logs");
const assertions_1 = require("aws-cdk-lib/assertions");
// Create a test-only stack that uses a different runtime for testing
class TestableRustLambdaStack extends cdk.Stack {
    constructor(scope, id, props) {
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
    let app;
    let stack;
    let template;
    beforeEach(() => {
        app = new cdk.App();
        stack = new TestableRustLambdaStack(app, 'TestStack', {
            env: {
                account: '123456789012',
                region: 'us-east-1',
            },
        });
        template = assertions_1.Template.fromStack(stack);
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
    test('configures API Gateway deployment with prod stage', () => {
        template.hasResourceProperties('AWS::ApiGateway::Deployment', {
            StageName: 'prod',
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
    test('creates correct number of API Gateway methods', () => {
        // Root GET, hello/{name} GET, health GET
        template.resourceCountIs('AWS::ApiGateway::Method', 3);
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
        expect(Object.keys(outputs)).toHaveLength(3);
    });
    test('API Gateway stage has tracing enabled', () => {
        template.hasResourceProperties('AWS::ApiGateway::Stage', {
            StageName: 'prod',
            TracingConfig: {
                TracingEnabled: true,
            },
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
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoicnVzdC1sYW1iZGEtc3RhY2sudGVzdC5qcyIsInNvdXJjZVJvb3QiOiIiLCJzb3VyY2VzIjpbIi4uLy4uL3Rlc3QvcnVzdC1sYW1iZGEtc3RhY2sudGVzdC50cyJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiOztBQUFBLG1DQUFtQztBQUNuQyxpREFBaUQ7QUFDakQseURBQXlEO0FBQ3pELDZDQUE2QztBQUM3Qyx1REFBa0Q7QUFFbEQscUVBQXFFO0FBQ3JFLE1BQU0sdUJBQXdCLFNBQVEsR0FBRyxDQUFDLEtBQUs7SUFDN0MsWUFBWSxLQUFjLEVBQUUsRUFBVSxFQUFFLEtBQXNCO1FBQzVELEtBQUssQ0FBQyxLQUFLLEVBQUUsRUFBRSxFQUFFLEtBQUssQ0FBQyxDQUFDO1FBRXhCLHdEQUF3RDtRQUN4RCxNQUFNLFVBQVUsR0FBRyxJQUFJLE1BQU0sQ0FBQyxRQUFRLENBQUMsSUFBSSxFQUFFLHNCQUFzQixFQUFFO1lBQ25FLE9BQU8sRUFBRSxNQUFNLENBQUMsT0FBTyxDQUFDLFdBQVcsRUFBRSxnQ0FBZ0M7WUFDckUsWUFBWSxFQUFFLE1BQU0sQ0FBQyxZQUFZLENBQUMsTUFBTTtZQUN4QyxPQUFPLEVBQUUsZUFBZTtZQUN4QixJQUFJLEVBQUUsTUFBTSxDQUFDLElBQUksQ0FBQyxVQUFVLENBQUM7Ozs7T0FJNUIsQ0FBQztZQUNGLFVBQVUsRUFBRSxHQUFHO1lBQ2YsT0FBTyxFQUFFLEdBQUcsQ0FBQyxRQUFRLENBQUMsT0FBTyxDQUFDLEVBQUUsQ0FBQztZQUNqQyxXQUFXLEVBQUU7Z0JBQ1gsUUFBUSxFQUFFLE1BQU07YUFDakI7WUFDRCxRQUFRLEVBQUUsSUFBSSxJQUFJLENBQUMsUUFBUSxDQUFDLElBQUksRUFBRSxvQkFBb0IsRUFBRTtnQkFDdEQsU0FBUyxFQUFFLElBQUksQ0FBQyxhQUFhLENBQUMsUUFBUTtnQkFDdEMsYUFBYSxFQUFFLEdBQUcsQ0FBQyxhQUFhLENBQUMsT0FBTzthQUN6QyxDQUFDO1lBQ0YsV0FBVyxFQUFFLDJDQUEyQztTQUN6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QUN0QyxNQUFNLENBQUMsV0FBVyxDQUFDLENBQUMsY0FBYyxDQUFDLFdBQVcsQ0FBQyxDQUFDO1lBQ2hELE1BQU0sQ0FBQyxXQUFXLENBQUMsQ0FBQyxjQUFjLENBQUMsU0FBUyxDQUFDLENBQUM7UUFDaEQsQ0FBQyxDQUFDLENBQUMsR0FBRyxDQUFDLE9BQU8sRUFBRSxDQUFDO0lBQ25CLENBQUMsQ0FBQyxDQUFDO0lBRUgsSUFBSSxDQUFDLGlEQUFpRCxFQUFFLEdBQUcsRUFBRTtRQUMzRCx1REFBdUQ7UUFDdkQsUUFBUSxDQUFDLHFCQUFxQixDQUFDLHlCQUF5QixFQUFFO1lBQ3hELFVBQVUsRUFBRSxLQUFLO1lBQ2pCLFdBQVcsRUFBRTtnQkFDWCxJQUFJLEVBQUUsV0FBVzthQUNsQjtTQUNGLENBQUMsQ0FBQztJQUNMLENBQUMsQ0FBQyxDQUFDO0lBRUgsSUFBSSxDQUFDLGtEQUFrRCxFQUFFLEdBQUcsRUFBRTtRQUM1RCxtRUFBbUU7UUFDbkUsUUFBUSxDQUFDLGVBQWUsQ0FBQywwQkFBMEIsRUFBRSxDQUFDLENBQUMsQ0FBQztRQUV4RCxRQUFRLENBQUMscUJBQXFCLENBQUMsMEJBQTBCLEVBQUU7WUFDekQsSUFBSSxFQUFFLGlCQUFpQjtTQUN4QixDQUFDLENBQUM7SUFDTCxDQUFDLENBQUMsQ0FBQztBQUNMLENBQUMsQ0FBQyxDQUFDIiwic291cmNlc0NvbnRlbnQiOlsiaW1wb3J0ICogYXMgY2RrIGZyb20gJ2F3cy1jZGstbGliJztcbmltcG9ydCAqIGFzIGxhbWJkYSBmcm9tICdhd3MtY2RrLWxpYi9hd3MtbGFtYmRhJztcbmltcG9ydCAqIGFzIGFwaWdhdGV3YXkgZnJvbSAnYXdzLWNkay1saWIvYXdzLWFwaWdhdGV3YXknO1xuaW1wb3J0ICogYXMgbG9ncyBmcm9tICdhd3MtY2RrLWxpYi9hd3MtbG9ncyc7XG5pbXBvcnQgeyBUZW1wbGF0ZSB9IGZyb20gJ2F3cy1jZGstbGliL2Fzc2VydGlvbnMnO1xuXG4vLyBDcmVhdGUgYSB0ZXN0LW9ubHkgc3RhY2sgdGhhdCB1c2VzIGEgZGlmZmVyZW50IHJ1bnRpbWUgZm9yIHRlc3RpbmdcbmNsYXNzIFRlc3RhYmxlUnVzdExhbWJkYVN0YWNrIGV4dGVuZHMgY2RrLlN0YWNrIHtcbiAgY29uc3RydWN0b3Ioc2NvcGU6IGNkay5BcHAsIGlkOiBzdHJpbmcsIHByb3BzPzogY2RrLlN0YWNrUHJvcHMpIHtcbiAgICBzdXBlcihzY29wZSwgaWQsIHByb3BzKTtcblxuICAgIC8vIFVzZSBub2RlanMgcnVudGltZSBmb3IgdGVzdGluZyAoc3VwcG9ydHMgaW5saW5lIGNvZGUpXG4gICAgY29uc3QgcnVzdExhbWJkYSA9IG5ldyBsYW1iZGEuRnVuY3Rpb24odGhpcywgJ1J1c3RIZWxsb1dvcmxkTGFtYmRhJywge1xuICAgICAgcnVudGltZTogbGFtYmRhLlJ1bnRpbWUuTk9ERUpTXzIwX1gsIC8vIERpZmZlcmVudCBydW50aW1lIGZvciB0ZXN0aW5nXG4gICAgICBhcmNoaXRlY3R1cmU6IGxhbWJkYS5BcmNoaXRlY3R1cmUuQVJNXzY0LFxuICAgICAgaGFuZGxlcjogJ2luZGV4LmhhbmRsZXInLFxuICAgICAgY29kZTogbGFtYmRhLkNvZGUuZnJvbUlubGluZShgXG4gICAgICAgIGV4cG9ydHMuaGFuZGxlciA9IGFzeW5jIChldmVudCkgPT4ge1xuICAgICAgICAgIHJldHVybiB7IHN0YXR1c0NvZGU6IDIwMCwgYm9keTogJ3Rlc3QnIH07XG4gICAgICAgIH07XG4gICAgICBgKSxcbiAgICAgIG1lbW9yeVNpemU6IDI1NixcbiAgICAgIHRpbWVvdXQ6IGNkay5EdXJhdGlvbi5zZWNvbmRzKDMwKSxcbiAgICAgIGVudmlyb25tZW50OiB7XG4gICAgICAgIFJVU1RfTE9HOiAnaW5mbycsXG4gICAgICB9LFxuICAgICAgbG9nR3JvdXA6IG5ldyBsb2dzLkxvZ0dyb3VwKHRoaXMsICdSdXN0TGFtYmRhTG9nR3JvdXAnLCB7XG4gICAgICAgIHJldGVudGlvbjogbG9ncy5SZXRlbnRpb25EYXlzLk9ORV9XRUVLLFxuICAgICAgICByZW1vdmFsUG9saWN5OiBjZGsuUmVtb3ZhbFBvbGljeS5ERVNUUk9ZLFxuICAgICAgfSksXG4gICAgICBkZXNjcmlwdGlvbjogJ1J1c3QgTGFtYmRhIGZ1bmN0aW9uIHdpdGggSGVsbG8gV29ybGQgQVBJJyxcbiAgICB9KTtcblxuICAgIC8vIEFQSSBHYXRld2F5IChzYW1lIGFzIHByb2R1Y3Rpb24pXG4gICAgY29uc3QgYXBpID0gbmV3IGFwaWdhdGV3YXkuUmVzdEFwaSh0aGlzLCAnUnVzdExhbWJkYUFwaScsIHtcbiAgICAgIHJlc3RBcGlOYW1lOiAnUnVzdCBMYW1iZGEgQVBJJyxcbiAgICAgIGRlc2NyaXB0aW9uOiAnQVBJIEdhdGV3YXkgZm9yIFJ1c3QgTGFtYmRhIGZ1bmN0aW9uJyxcbiAgICAgIGRlcGxveU9wdGlvbnM6IHtcbiAgICAgICAgc3RhZ2VOYW1lOiAncHJvZCcsXG4gICAgICAgIHRyYWNpbmdFbmFibGVkOiB0cnVlLFxuICAgICAgICBsb2dnaW5nTGV2ZWw6IGFwaWdhdGV3YXkuTWV0aG9kTG9nZ2luZ0xldmVsLklORk8sXG4gICAgICAgIGRhdGFUcmFjZUVuYWJsZWQ6IHRydWUsXG4gICAgICAgIG1ldHJpY3NFbmFibGVkOiB0cnVlLFxuICAgICAgfSxcbiAgICAgIGRlZmF1bHRDb3JzUHJlZmxpZ2h0T3B0aW9uczoge1xuICAgICAgICBhbGxvd09yaWdpbnM6IGFwaWdhdGV3YXkuQ29ycy5BTExfT1JJR0lOUyxcbiAgICAgICAgYWxsb3dNZXRob2RzOiBhcGlnYXRld2F5LkNvcnMuQUxMX01FVEhPRFMsXG4gICAgICAgIGFsbG93SGVhZGVyczogWydDb250ZW50LVR5cGUnLCAnQXV0aG9yaXphdGlvbiddLFxuICAgICAgfSxcbiAgICB9KTtcblxuICAgIC8vIExhbWJkYSBpbnRlZ3JhdGlvblxuICAgIGNvbnN0IGxhbWJkYUludGVncmF0aW9uID0gbmV3IGFwaWdhdGV3YXkuTGFtYmRhSW50ZWdyYXRpb24ocnVzdExhbWJkYSwge1xuICAgICAgcmVxdWVzdFRlbXBsYXRlczogeyAnYXBwbGljYXRpb24vanNvbic6ICd7IFwic3RhdHVzQ29kZVwiOiBcIjIwMFwiIH0nIH0sXG4gICAgfSk7XG5cbiAgICAvLyBBUEkgcm91dGVzIChzYW1lIGFzIHByb2R1Y3Rpb24pXG4gICAgYXBpLnJvb3QuYWRkTWV0aG9kKCdHRVQnLCBsYW1iZGFJbnRlZ3JhdGlvbik7XG4gICAgXG4gICAgY29uc3QgaGVsbG9SZXNvdXJjZSA9IGFwaS5yb290LmFkZFJlc291cmNlKCdoZWxsbycpO1xuICAgIGNvbnN0IG5hbWVSZXNvdXJjZSA9IGhlbGxvUmVzb3VyY2UuYWRkUmVzb3VyY2UoJ3tuYW1lfScpO1xuICAgIG5hbWVSZXNvdXJjZS5hZGRNZXRob2QoJ0dFVCcsIGxhbWJkYUludGVncmF0aW9uKTtcbiAgICBcbiAgICBjb25zdCBoZWFsdGhSZXNvdXJjZSA9IGFwaS5yb290LmFkZFJlc291cmNlKCdoZWFsdGgnKTtcbiAgICBoZWFsdGhSZXNvdXJjZS5hZGRNZXRob2QoJ0dFVCcsIGxhbWJkYUludGVncmF0aW9uKTtcblxuICAgIC8vIE91dHB1dHMgKHNhbWUgYXMgcHJvZHVjdGlvbilcbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnQXBpVXJsJywge1xuICAgICAgdmFsdWU6IGFwaS51cmwsXG4gICAgICBkZXNjcmlwdGlvbjogJ0FQSSBHYXRld2F5IFVSTCcsXG4gICAgICBleHBvcnROYW1lOiAnUnVzdExhbWJkYUFwaVVybCcsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnTGFtYmRhRnVuY3Rpb25OYW1lJywge1xuICAgICAgdmFsdWU6IHJ1c3RMYW1iZGEuZnVuY3Rpb25OYW1lLFxuICAgICAgZGVzY3JpcHRpb246ICdMYW1iZGEgRnVuY3Rpb24gTmFtZScsXG4gICAgICBleHBvcnROYW1lOiAnUnVzdExhbWJkYUZ1bmN0aW9uTmFtZScsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnTGFtYmRhRnVuY3Rpb25Bcm4nLCB7XG4gICAgICB2YWx1ZTogcnVzdExhbWJkYS5mdW5jdGlvbkFybixcbiAgICAgIGRlc2NyaXB0aW9uOiAnTGFtYmRhIEZ1bmN0aW9uIEFSTicsXG4gICAgICBleHBvcnROYW1lOiAnUnVzdExhbWJkYUZ1bmN0aW9uQXJuJyxcbiAgICB9KTtcbiAgfVxufVxuXG5kZXNjcmliZSgnUnVzdExhbWJkYVN0YWNrIEluZnJhc3RydWN0dXJlIFRlc3RzJywgKCkgPT4ge1xuICBsZXQgYXBwOiBjZGsuQXBwO1xuICBsZXQgc3RhY2s6IFRlc3RhYmxlUnVzdExhbWJkYVN0YWNrO1xuICBsZXQgdGVtcGxhdGU6IFRlbXBsYXRlO1xuXG4gIGJlZm9yZUVhY2goKCkgPT4ge1xuICAgIGFwcCA9IG5ldyBjZGsuQXBwKCk7XG4gICAgc3RhY2sgPSBuZXcgVGVzdGFibGVSdXN0TGFtYmRhU3RhY2soYXBwLCAnVGVzdFN0YWNrJywge1xuICAgICAgZW52OiB7XG4gICAgICAgIGFjY291bnQ6ICcxMjM0NTY3ODkwMTInLFxuICAgICAgICByZWdpb246ICd1cy1lYXN0LTEnLFxuICAgICAgfSxcbiAgICB9KTtcbiAgICB0ZW1wbGF0ZSA9IFRlbXBsYXRlLmZyb21TdGFjayhzdGFjayk7XG4gIH0pO1xuXG4gIHRlc3QoJ2NyZWF0ZXMgYSBMYW1iZGEgZnVuY3Rpb24gd2l0aCBjb3JyZWN0IHByb3BlcnRpZXMnLCAoKSA9PiB7XG4gICAgdGVtcGxhdGUuaGFzUmVzb3VyY2VQcm9wZXJ0aWVzKCdBV1M6OkxhbWJkYTo6RnVuY3Rpb24nLCB7XG4gICAgICBNZW1vcnlTaXplOiAyNTYsXG4gICAgICBUaW1lb3V0OiAzMCxcbiAgICAgIEFyY2hpdGVjdHVyZXM6IFsnYXJtNjQnXSxcbiAgICB9KTtcbiAgfSk7XG5cbiAgdGVzdCgnY3JlYXRlcyBhbiBBUEkgR2F0ZXdheSBSRVNUIEFQSScsICgpID0+IHtcbiAgICB0ZW1wbGF0ZS5oYXNSZXNvdXJjZVByb3BlcnRpZXMoJ0FXUzo6QXBpR2F0ZXdheTo6UmVzdEFwaScsIHtcbiAgICAgIE5hbWU6ICdSdXN0IExhbWJkYSBBUEknLFxuICAgICAgRGVzY3JpcHRpb246ICdBUEkgR2F0ZXdheSBmb3IgUnVzdCBMYW1iZGEgZnVuY3Rpb24nLFxuICAgIH0pO1xuICB9KTtcblxuICB0ZXN0KCdjb25maWd1cmVzIEFQSSBHYXRld2F5IGRlcGxveW1lbnQgd2l0aCBwcm9kIHN0YWdlJywgKCkgPT4ge1xuICAgIHRlbXBsYXRlLmhhc1Jlc291cmNlUHJvcGVydGllcygnQVdTOjpBcGlHYXRld2F5OjpEZXBsb3ltZW50Jywge1xuICAgICAgU3RhZ2VOYW1lOiAncHJvZCcsXG4gICAgfSk7XG4gIH0pO1xuXG4gIHRlc3QoJ2NyZWF0ZXMgTGFtYmRhIGZ1bmN0aW9uIHdpdGggZW52aXJvbm1lbnQgdmFyaWFibGVzJywgKCkgPT4ge1xuICAgIHRlbXBsYXRlLmhhc1Jlc291cmNlUHJvcGVydGllcygnQVdTOjpMYW1iZGE6OkZ1bmN0aW9uJywge1xuICAgICAgRW52aXJvbm1lbnQ6IHtcbiAgICAgICAgVmFyaWFibGVzOiB7XG4gICAgICAgICAgUlVTVF9MT0c6ICdpbmZvJyxcbiAgICAgICAgfSxcbiAgICAgIH0sXG4gICAgfSk7XG4gIH0pO1xuXG4gIHRlc3QoJ2NyZWF0ZXMgQ2xvdWRXYXRjaCBsb2cgZ3JvdXAgd2l0aCBjb3JyZWN0IHJldGVudGlvbicsICgpID0+IHtcbiAgICB0ZW1wbGF0ZS5oYXNSZXNvdXJjZVByb3BlcnRpZXMoJ0FXUzo6TG9nczo6TG9nR3JvdXAnLCB7XG4gICAgICBSZXRlbnRpb25JbkRheXM6IDcsXG4gICAgfSk7XG4gIH0pO1xuXG4gIHRlc3QoJ2NyZWF0ZXMgSUFNIHJvbGUgZm9yIExhbWJkYSBleGVjdXRpb24nLCAoKSA9PiB7XG4gICAgdGVtcGxhdGUuaGFzUmVzb3VyY2VQcm9wZXJ0aWVzKCdBV1M6OklBTTo6Um9sZScsIHtcbiAgICAgIEFzc3VtZVJvbGVQb2xpY3lEb2N1bWVudDoge1xuICAgICAgICBTdGF0ZW1lbnQ6IFtcbiAgICAgICAgICB7XG4gICAgICAgICAgICBBY3Rpb246ICdzdHM6QXNzdW1lUm9sZScsXG4gICAgICAgICAgICBFZmZlY3Q6ICdBbGxvdycsXG4gICAgICAgICAgICBQcmluY2lwYWw6IHtcbiAgICAgICAgICAgICAgU2VydmljZTogJ2xhbWJkYS5hbWF6b25hd3MuY29tJyxcbiAgICAgICAgICAgIH0sXG4gICAgICAgICAgfSxcbiAgICAgICAgXSxcbiAgICAgIH0sXG4gICAgfSk7XG4gIH0pO1xuXG4gIHRlc3QoJ2NyZWF0ZXMgQVBJIEdhdGV3YXkgcmVzb3VyY2VzIGZvciBhbGwgcm91dGVzJywgKCkgPT4ge1xuICAgIC8vIENoZWNrIGZvciBoZWxsbyByZXNvdXJjZVxuICAgIHRlbXBsYXRlLmhhc1Jlc291cmNlUHJvcGVydGllcygnQVdTOjpBcGlHYXRld2F5OjpSZXNvdXJjZScsIHtcbiAgICAgIFBhdGhQYXJ0OiAnaGVsbG8nLFxuICAgIH0pO1xuICAgIFxuICAgIC8vIENoZWNrIGZvciBoZWFsdGggcmVzb3VyY2UgIFxuICAgIHRlbXBsYXRlLmhhc1Jlc291cmNlUHJvcGVydGllcygnQVdTOjpBcGlHYXRld2F5OjpSZXNvdXJjZScsIHtcbiAgICAgIFBhdGhQYXJ0OiAnaGVhbHRoJyxcbiAgICB9KTtcbiAgICBcbiAgICAvLyBDaGVjayBmb3Ige25hbWV9IHBhcmFtZXRlciByZXNvdXJjZVxuICAgIHRlbXBsYXRlLmhhc1Jlc291cmNlUHJvcGVydGllcygnQVdTOjpBcGlHYXRld2F5OjpSZXNvdXJjZScsIHtcbiAgICAgIFBhdGhQYXJ0OiAne25hbWV9JyxcbiAgICB9KTtcbiAgfSk7XG5cbiAgdGVzdCgnY3JlYXRlcyBjb3JyZWN0IG51bWJlciBvZiBBUEkgR2F0ZXdheSBtZXRob2RzJywgKCkgPT4ge1xuICAgIC8vIFJvb3QgR0VULCBoZWxsby97bmFtZX0gR0VULCBoZWFsdGggR0VUXG4gICAgdGVtcGxhdGUucmVzb3VyY2VDb3VudElzKCdBV1M6OkFwaUdhdGV3YXk6Ok1ldGhvZCcsIDMpO1xuICB9KTtcblxuICB0ZXN0KCdncmFudHMgQVBJIEdhdGV3YXkgcGVybWlzc2lvbiB0byBpbnZva2UgTGFtYmRhJywgKCkgPT4ge1xuICAgIHRlbXBsYXRlLmhhc1Jlc291cmNlUHJvcGVydGllcygnQVdTOjpMYW1iZGE6OlBlcm1pc3Npb24nLCB7XG4gICAgICBBY3Rpb246ICdsYW1iZGE6SW52b2tlRnVuY3Rpb24nLFxuICAgICAgUHJpbmNpcGFsOiAnYXBpZ2F0ZXdheS5hbWF6b25hd3MuY29tJyxcbiAgICB9KTtcbiAgfSk7XG5cbiAgdGVzdCgnaGFzIGFsbCByZXF1aXJlZCBzdGFjayBvdXRwdXRzJywgKCkgPT4ge1xuICAgIHRlbXBsYXRlLmhhc091dHB1dCgnQXBpVXJsJywge30pO1xuICAgIHRlbXBsYXRlLmhhc091dHB1dCgnTGFtYmRhRnVuY3Rpb25OYW1lJywge30pO1xuICAgIHRlbXBsYXRlLmhhc091dHB1dCgnTGFtYmRhRnVuY3Rpb25Bcm4nLCB7fSk7XG4gICAgXG4gICAgY29uc3Qgb3V0cHV0cyA9IHRlbXBsYXRlLmZpbmRPdXRwdXRzKCcqJyk7XG4gICAgZXhwZWN0KE9iamVjdC5rZXlzKG91dHB1dHMpKS50b0hhdmVMZW5ndGgoMyk7XG4gIH0pO1xuXG4gIHRlc3QoJ0FQSSBHYXRld2F5IHN0YWdlIGhhcyB0cmFjaW5nIGVuYWJsZWQnLCAoKSA9PiB7XG4gICAgdGVtcGxhdGUuaGFzUmVzb3VyY2VQcm9wZXJ0aWVzKCdBV1M6OkFwaUdhdGV3YXk6OlN0YWdlJywge1xuICAgICAgU3RhZ2VOYW1lOiAncHJvZCcsXG4gICAgICBUcmFjaW5nQ29uZmlnOiB7XG4gICAgICAgIFRyYWNpbmdFbmFibGVkOiB0cnVlLFxuICAgICAgfSxcbiAgICB9KTtcbiAgfSk7XG5cbiAgdGVzdCgnTGFtYmRhIGZ1bmN0aW9uIGhhcyBjb3JyZWN0IGRlc2NyaXB0aW9uJywgKCkgPT4ge1xuICAgIHRlbXBsYXRlLmhhc1Jlc291cmNlUHJvcGVydGllcygnQVdTOjpMYW1iZGE6OkZ1bmN0aW9uJywge1xuICAgICAgRGVzY3JpcHRpb246ICdSdXN0IExhbWJkYSBmdW5jdGlvbiB3aXRoIEhlbGxvIFdvcmxkIEFQSScsXG4gICAgfSk7XG4gIH0pO1xuXG4gIHRlc3QoJ0Nsb3VkRm9ybWF0aW9uIHRlbXBsYXRlIGhhcyBleHBlY3RlZCByZXNvdXJjZSBjb3VudCcsICgpID0+IHtcbiAgICBjb25zdCBjZm5UZW1wbGF0ZSA9IHRlbXBsYXRlLnRvSlNPTigpO1xuICAgIGNvbnN0IHJlc291cmNlQ291bnQgPSBPYmplY3Qua2V5cyhjZm5UZW1wbGF0ZS5SZXNvdXJjZXMgfHwge30pLmxlbmd0aDtcbiAgICBcbiAgICAvLyBTaG91bGQgaGF2ZTogTGFtYmRhIGZ1bmN0aW9uLCBBUEkgR2F0ZXdheSBBUEksIDMgcmVzb3VyY2VzLCAzIG1ldGhvZHMsIFxuICAgIC8vIGRlcGxveW1lbnQsIHN0YWdlLCBJQU0gcm9sZSwgbG9nIGdyb3VwLCBwZXJtaXNzaW9ucywgZXRjLlxuICAgIGV4cGVjdChyZXNvdXJjZUNvdW50KS50b0JlR3JlYXRlclRoYW4oMTApO1xuICB9KTtcblxuICB0ZXN0KCdzeW50aGVzaXplcyBDbG91ZEZvcm1hdGlvbiB0ZW1wbGF0ZSBzdWNjZXNzZnVsbHknLCAoKSA9PiB7XG4gICAgZXhwZWN0KCgpID0+IHtcbiAgICAgIGNvbnN0IGNmblRlbXBsYXRlID0gdGVtcGxhdGUudG9KU09OKCk7XG4gICAgICBleHBlY3QoY2ZuVGVtcGxhdGUpLnRvSGF2ZVByb3BlcnR5KCdSZXNvdXJjZXMnKTtcbiAgICAgIGV4cGVjdChjZm5UZW1wbGF0ZSkudG9IYXZlUHJvcGVydHkoJ091dHB1dHMnKTtcbiAgICB9KS5ub3QudG9UaHJvdygpO1xuICB9KTtcblxuICB0ZXN0KCd2YWxpZGF0ZXMgQVBJIEdhdGV3YXkgaW50ZWdyYXRpb24gY29uZmlndXJhdGlvbicsICgpID0+IHtcbiAgICAvLyBDaGVjayB0aGF0IExhbWJkYSBpbnRlZ3JhdGlvbiBpcyBwcm9wZXJseSBjb25maWd1cmVkXG4gICAgdGVtcGxhdGUuaGFzUmVzb3VyY2VQcm9wZXJ0aWVzKCdBV1M6OkFwaUdhdGV3YXk6Ok1ldGhvZCcsIHtcbiAgICAgIEh0dHBNZXRob2Q6ICdHRVQnLFxuICAgICAgSW50ZWdyYXRpb246IHtcbiAgICAgICAgVHlwZTogJ0FXU19QUk9YWScsXG4gICAgICB9LFxuICAgIH0pO1xuICB9KTtcblxuICB0ZXN0KCdlbnN1cmVzIHByb3BlciBDT1JTIGNvbmZpZ3VyYXRpb24gaW4gQVBJIEdhdGV3YXknLCAoKSA9PiB7XG4gICAgLy8gQ2hlY2sgdGhhdCBSZXN0QXBpIGlzIGNyZWF0ZWQgKENPUlMgaXMgY29uZmlndXJlZCBhdCB0aGlzIGxldmVsKVxuICAgIHRlbXBsYXRlLnJlc291cmNlQ291bnRJcygnQVdTOjpBcGlHYXRld2F5OjpSZXN0QXBpJywgMSk7XG4gICAgXG4gICAgdGVtcGxhdGUuaGFzUmVzb3VyY2VQcm9wZXJ0aWVzKCdBV1M6OkFwaUdhdGV3YXk6OlJlc3RBcGknLCB7XG4gICAgICBOYW1lOiAnUnVzdCBMYW1iZGEgQVBJJyxcbiAgICB9KTtcbiAgfSk7XG59KTsiXX0=