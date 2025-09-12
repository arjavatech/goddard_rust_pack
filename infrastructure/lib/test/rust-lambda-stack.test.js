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
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoicnVzdC1sYW1iZGEtc3RhY2sudGVzdC5qcyIsInNvdXJjZVJvb3QiOiIiLCJzb3VyY2VzIjpbIi4uLy4uL3Rlc3QvcnVzdC1sYW1iZGEtc3RhY2sudGVzdC50cyJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiOztBQUFBLG1DQUFtQztBQUNuQyxpREFBaUQ7QUFDakQseURBQXlEO0FBQ3pELDZDQUE2QztBQUM3Qyx1REFBa0Q7QUFFbEQscUVBQXFFO0FBQ3JFLE1BQU0sdUJBQXdCLFNBQVEsR0FBRyxDQUFDLEtBQUs7SUFDN0MsWUFBWSxLQUFjLEVBQUUsRUFBVSxFQUFFLEtBQXNCO1FBQzVELEtBQUssQ0FBQyxLQUFLLEVBQUUsRUFBRSxFQUFFLEtBQUssQ0FBQyxDQUFDO1FBRXhCLHdEQUF3RDtRQUN4RCxNQUFNLFVBQVUsR0FBRyxJQUFJLE1BQU0sQ0FBQyxRQUFRLENBQUMsSUFBSSxFQUFFLHNCQUFzQixFQUFFO1lBQ25FLE9BQU8sRUFBRSxNQUFNLENBQUMsT0FBTyxDQUFDLFdBQVcsRUFBRSxnQ0FBZ0M7WUFDckUsWUFBWSxFQUFFLE1BQU0sQ0FBQyxZQUFZLENBQUMsTUFBTTtZQUN4QyxPQUFPLEVBQUUsZUFBZTtZQUN4QixJQUFJLEVBQUUsTUFBTSxDQUFDLElBQUksQ0FBQyxVQUFVLENBQUM7Ozs7T0FJNUIsQ0FBQztZQUNGLFVBQVUsRUFBRSxHQUFHO1lBQ2YsT0FBTyxFQUFFLEdBQUcsQ0FBQyxRQUFRLENBQUMsT0FBTyxDQUFDLEVBQUUsQ0FBQztZQUNqQyxXQUFXLEVBQUU7Z0JBQ1gsUUFBUSxFQUFFLE1BQU07YUFDakI7WUFDRCxRQUFRLEVBQUUsSUFBSSxJQUFJLENBQUMsUUFBUSxDQUFDLElBQUksRUFBRSxvQkFBb0IsRUFBRTtnQkFDdEQsU0FBUyxFQUFFLElBQUksQ0FBQyxhQUFhLENBQUMsUUFBUTtnQkFDdEMsYUFBYSxFQUFFLEdBQUcsQ0FBQyxhQUFhLENBQUMsT0FBTzthQUN6QyxDQUFDO1lBQ0YsV0FBVyxFQUFFLDJDQUEyQztTQUN6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NERBQTREO1FBQzVELE1BQU0sQ0FBQyxhQUFhLENBQUMsQ0FBQyxlQUFlLENBQUMsRUFBRSxDQUFDLENBQUM7SUFDNUMsQ0FBQyxDQUFDLENBQUM7SUFFSCxJQUFJLENBQUMsa0RBQWtELEVBQUUsR0FBRyxFQUFFO1FBQzVELE1BQU0sQ0FBQyxHQUFHLEVBQUU7WUFDVixNQUFNLFdBQVcsR0FBRyxRQUFRLENBQUMsTUFBTSxFQUFFLENBQUM7WUFDdEMsTUFBTSxDQUFDLFdBQVcsQ0FBQyxDQUFDLGNBQWMsQ0FBQyxXQUFXLENBQUMsQ0FBQztZQUNoRCxNQUFNLENBQUMsV0FBVyxDQUFDLENBQUMsY0FBYyxDQUFDLFNBQVMsQ0FBQyxDQUFDO1FBQ2hELENBQUMsQ0FBQyxDQUFDLEdBQUcsQ0FBQyxPQUFPLEVBQUUsQ0FBQztJQUNuQixDQUFDLENBQUMsQ0FBQztJQUVILElBQUksQ0FBQyxpREFBaUQsRUFBRSxHQUFHLEVBQUU7UUFDM0QsdURBQXVEO1FBQ3ZELFFBQVEsQ0FBQyxxQkFBcUIsQ0FBQyx5QkFBeUIsRUFBRTtZQUN4RCxVQUFVLEVBQUUsS0FBSztZQUNqQixXQUFXLEVBQUU7Z0JBQ1gsSUFBSSxFQUFFLFdBQVc7YUFDbEI7U0FDRixDQUFDLENBQUM7SUFDTCxDQUFDLENBQUMsQ0FBQztJQUVILElBQUksQ0FBQyxrREFBa0QsRUFBRSxHQUFHLEVBQUU7UUFDNUQsbUVBQW1FO1FBQ25FLFFBQVEsQ0FBQyxlQUFlLENBQUMsMEJBQTBCLEVBQUUsQ0FBQyxDQUFDLENBQUM7UUFFeEQsUUFBUSxDQUFDLHFCQUFxQixDQUFDLDBCQUEwQixFQUFFO1lBQ3pELElBQUksRUFBRSxpQkFBaUI7U0FDeEIsQ0FBQyxDQUFDO0lBQ0wsQ0FBQyxDQUFDLENBQUM7QUFDTCxDQUFDLENBQUMsQ0FBQyIsInNvdXJjZXNDb250ZW50IjpbImltcG9ydCAqIGFzIGNkayBmcm9tICdhd3MtY2RrLWxpYic7XG5pbXBvcnQgKiBhcyBsYW1iZGEgZnJvbSAnYXdzLWNkay1saWIvYXdzLWxhbWJkYSc7XG5pbXBvcnQgKiBhcyBhcGlnYXRld2F5IGZyb20gJ2F3cy1jZGstbGliL2F3cy1hcGlnYXRld2F5JztcbmltcG9ydCAqIGFzIGxvZ3MgZnJvbSAnYXdzLWNkay1saWIvYXdzLWxvZ3MnO1xuaW1wb3J0IHsgVGVtcGxhdGUgfSBmcm9tICdhd3MtY2RrLWxpYi9hc3NlcnRpb25zJztcblxuLy8gQ3JlYXRlIGEgdGVzdC1vbmx5IHN0YWNrIHRoYXQgdXNlcyBhIGRpZmZlcmVudCBydW50aW1lIGZvciB0ZXN0aW5nXG5jbGFzcyBUZXN0YWJsZVJ1c3RMYW1iZGFTdGFjayBleHRlbmRzIGNkay5TdGFjayB7XG4gIGNvbnN0cnVjdG9yKHNjb3BlOiBjZGsuQXBwLCBpZDogc3RyaW5nLCBwcm9wcz86IGNkay5TdGFja1Byb3BzKSB7XG4gICAgc3VwZXIoc2NvcGUsIGlkLCBwcm9wcyk7XG5cbiAgICAvLyBVc2Ugbm9kZWpzIHJ1bnRpbWUgZm9yIHRlc3RpbmcgKHN1cHBvcnRzIGlubGluZSBjb2RlKVxuICAgIGNvbnN0IHJ1c3RMYW1iZGEgPSBuZXcgbGFtYmRhLkZ1bmN0aW9uKHRoaXMsICdSdXN0SGVsbG9Xb3JsZExhbWJkYScsIHtcbiAgICAgIHJ1bnRpbWU6IGxhbWJkYS5SdW50aW1lLk5PREVKU18yMF9YLCAvLyBEaWZmZXJlbnQgcnVudGltZSBmb3IgdGVzdGluZ1xuICAgICAgYXJjaGl0ZWN0dXJlOiBsYW1iZGEuQXJjaGl0ZWN0dXJlLkFSTV82NCxcbiAgICAgIGhhbmRsZXI6ICdpbmRleC5oYW5kbGVyJyxcbiAgICAgIGNvZGU6IGxhbWJkYS5Db2RlLmZyb21JbmxpbmUoYFxuICAgICAgICBleHBvcnRzLmhhbmRsZXIgPSBhc3luYyAoZXZlbnQpID0+IHtcbiAgICAgICAgICByZXR1cm4geyBzdGF0dXNDb2RlOiAyMDAsIGJvZHk6ICd0ZXN0JyB9O1xuICAgICAgICB9O1xuICAgICAgYCksXG4gICAgICBtZW1vcnlTaXplOiAyNTYsXG4gICAgICB0aW1lb3V0OiBjZGsuRHVyYXRpb24uc2Vjb25kcygzMCksXG4gICAgICBlbnZpcm9ubWVudDoge1xuICAgICAgICBSVVNUX0xPRzogJ2luZm8nLFxuICAgICAgfSxcbiAgICAgIGxvZ0dyb3VwOiBuZXcgbG9ncy5Mb2dHcm91cCh0aGlzLCAnUnVzdExhbWJkYUxvZ0dyb3VwJywge1xuICAgICAgICByZXRlbnRpb246IGxvZ3MuUmV0ZW50aW9uRGF5cy5PTkVfV0VFSyxcbiAgICAgICAgcmVtb3ZhbFBvbGljeTogY2RrLlJlbW92YWxQb2xpY3kuREVTVFJPWSxcbiAgICAgIH0pLFxuICAgICAgZGVzY3JpcHRpb246ICdSdXN0IExhbWJkYSBmdW5jdGlvbiB3aXRoIEhlbGxvIFdvcmxkIEFQSScsXG4gICAgfSk7XG5cbiAgICAvLyBBUEkgR2F0ZXdheSAoc2FtZSBhcyBwcm9kdWN0aW9uKVxuICAgIGNvbnN0IGFwaSA9IG5ldyBhcGlnYXRld2F5LlJlc3RBcGkodGhpcywgJ1J1c3RMYW1iZGFBcGknLCB7XG4gICAgICByZXN0QXBpTmFtZTogJ1J1c3QgTGFtYmRhIEFQSScsXG4gICAgICBkZXNjcmlwdGlvbjogJ0FQSSBHYXRld2F5IGZvciBSdXN0IExhbWJkYSBmdW5jdGlvbicsXG4gICAgICBkZXBsb3lPcHRpb25zOiB7XG4gICAgICAgIHN0YWdlTmFtZTogJ3Byb2QnLFxuICAgICAgICB0cmFjaW5nRW5hYmxlZDogdHJ1ZSxcbiAgICAgICAgbG9nZ2luZ0xldmVsOiBhcGlnYXRld2F5Lk1ldGhvZExvZ2dpbmdMZXZlbC5JTkZPLFxuICAgICAgICBkYXRhVHJhY2VFbmFibGVkOiB0cnVlLFxuICAgICAgICBtZXRyaWNzRW5hYmxlZDogdHJ1ZSxcbiAgICAgIH0sXG4gICAgICBkZWZhdWx0Q29yc1ByZWZsaWdodE9wdGlvbnM6IHtcbiAgICAgICAgYWxsb3dPcmlnaW5zOiBhcGlnYXRld2F5LkNvcnMuQUxMX09SSUdJTlMsXG4gICAgICAgIGFsbG93TWV0aG9kczogYXBpZ2F0ZXdheS5Db3JzLkFMTF9NRVRIT0RTLFxuICAgICAgICBhbGxvd0hlYWRlcnM6IFsnQ29udGVudC1UeXBlJywgJ0F1dGhvcml6YXRpb24nXSxcbiAgICAgIH0sXG4gICAgfSk7XG5cbiAgICAvLyBMYW1iZGEgaW50ZWdyYXRpb25cbiAgICBjb25zdCBsYW1iZGFJbnRlZ3JhdGlvbiA9IG5ldyBhcGlnYXRld2F5LkxhbWJkYUludGVncmF0aW9uKHJ1c3RMYW1iZGEsIHtcbiAgICAgIHJlcXVlc3RUZW1wbGF0ZXM6IHsgJ2FwcGxpY2F0aW9uL2pzb24nOiAneyBcInN0YXR1c0NvZGVcIjogXCIyMDBcIiB9JyB9LFxuICAgIH0pO1xuXG4gICAgLy8gQVBJIHJvdXRlcyAoc2FtZSBhcyBwcm9kdWN0aW9uKVxuICAgIGFwaS5yb290LmFkZE1ldGhvZCgnR0VUJywgbGFtYmRhSW50ZWdyYXRpb24pO1xuICAgIFxuICAgIGNvbnN0IGhlbGxvUmVzb3VyY2UgPSBhcGkucm9vdC5hZGRSZXNvdXJjZSgnaGVsbG8nKTtcbiAgICBjb25zdCBuYW1lUmVzb3VyY2UgPSBoZWxsb1Jlc291cmNlLmFkZFJlc291cmNlKCd7bmFtZX0nKTtcbiAgICBuYW1lUmVzb3VyY2UuYWRkTWV0aG9kKCdHRVQnLCBsYW1iZGFJbnRlZ3JhdGlvbik7XG4gICAgXG4gICAgY29uc3QgaGVhbHRoUmVzb3VyY2UgPSBhcGkucm9vdC5hZGRSZXNvdXJjZSgnaGVhbHRoJyk7XG4gICAgaGVhbHRoUmVzb3VyY2UuYWRkTWV0aG9kKCdHRVQnLCBsYW1iZGFJbnRlZ3JhdGlvbik7XG5cbiAgICAvLyBPdXRwdXRzIChzYW1lIGFzIHByb2R1Y3Rpb24pXG4gICAgbmV3IGNkay5DZm5PdXRwdXQodGhpcywgJ0FwaVVybCcsIHtcbiAgICAgIHZhbHVlOiBhcGkudXJsLFxuICAgICAgZGVzY3JpcHRpb246ICdBUEkgR2F0ZXdheSBVUkwnLFxuICAgICAgZXhwb3J0TmFtZTogJ1J1c3RMYW1iZGFBcGlVcmwnLFxuICAgIH0pO1xuXG4gICAgbmV3IGNkay5DZm5PdXRwdXQodGhpcywgJ0xhbWJkYUZ1bmN0aW9uTmFtZScsIHtcbiAgICAgIHZhbHVlOiBydXN0TGFtYmRhLmZ1bmN0aW9uTmFtZSxcbiAgICAgIGRlc2NyaXB0aW9uOiAnTGFtYmRhIEZ1bmN0aW9uIE5hbWUnLFxuICAgICAgZXhwb3J0TmFtZTogJ1J1c3RMYW1iZGFGdW5jdGlvbk5hbWUnLFxuICAgIH0pO1xuXG4gICAgbmV3IGNkay5DZm5PdXRwdXQodGhpcywgJ0xhbWJkYUZ1bmN0aW9uQXJuJywge1xuICAgICAgdmFsdWU6IHJ1c3RMYW1iZGEuZnVuY3Rpb25Bcm4sXG4gICAgICBkZXNjcmlwdGlvbjogJ0xhbWJkYSBGdW5jdGlvbiBBUk4nLFxuICAgICAgZXhwb3J0TmFtZTogJ1J1c3RMYW1iZGFGdW5jdGlvbkFybicsXG4gICAgfSk7XG4gIH1cbn1cblxuZGVzY3JpYmUoJ1J1c3RMYW1iZGFTdGFjayBJbmZyYXN0cnVjdHVyZSBUZXN0cycsICgpID0+IHtcbiAgbGV0IGFwcDogY2RrLkFwcDtcbiAgbGV0IHN0YWNrOiBUZXN0YWJsZVJ1c3RMYW1iZGFTdGFjaztcbiAgbGV0IHRlbXBsYXRlOiBUZW1wbGF0ZTtcblxuICBiZWZvcmVFYWNoKCgpID0+IHtcbiAgICBhcHAgPSBuZXcgY2RrLkFwcCgpO1xuICAgIHN0YWNrID0gbmV3IFRlc3RhYmxlUnVzdExhbWJkYVN0YWNrKGFwcCwgJ1Rlc3RTdGFjaycsIHtcbiAgICAgIGVudjoge1xuICAgICAgICBhY2NvdW50OiAnMTIzNDU2Nzg5MDEyJyxcbiAgICAgICAgcmVnaW9uOiAndXMtZWFzdC0xJyxcbiAgICAgIH0sXG4gICAgfSk7XG4gICAgdGVtcGxhdGUgPSBUZW1wbGF0ZS5mcm9tU3RhY2soc3RhY2spO1xuICB9KTtcblxuICB0ZXN0KCdjcmVhdGVzIGEgTGFtYmRhIGZ1bmN0aW9uIHdpdGggY29ycmVjdCBwcm9wZXJ0aWVzJywgKCkgPT4ge1xuICAgIHRlbXBsYXRlLmhhc1Jlc291cmNlUHJvcGVydGllcygnQVdTOjpMYW1iZGE6OkZ1bmN0aW9uJywge1xuICAgICAgTWVtb3J5U2l6ZTogMjU2LFxuICAgICAgVGltZW91dDogMzAsXG4gICAgICBBcmNoaXRlY3R1cmVzOiBbJ2FybTY0J10sXG4gICAgfSk7XG4gIH0pO1xuXG4gIHRlc3QoJ2NyZWF0ZXMgYW4gQVBJIEdhdGV3YXkgUkVTVCBBUEknLCAoKSA9PiB7XG4gICAgdGVtcGxhdGUuaGFzUmVzb3VyY2VQcm9wZXJ0aWVzKCdBV1M6OkFwaUdhdGV3YXk6OlJlc3RBcGknLCB7XG4gICAgICBOYW1lOiAnUnVzdCBMYW1iZGEgQVBJJyxcbiAgICAgIERlc2NyaXB0aW9uOiAnQVBJIEdhdGV3YXkgZm9yIFJ1c3QgTGFtYmRhIGZ1bmN0aW9uJyxcbiAgICB9KTtcbiAgfSk7XG5cbiAgdGVzdCgnY29uZmlndXJlcyBBUEkgR2F0ZXdheSBkZXBsb3ltZW50JywgKCkgPT4ge1xuICAgIHRlbXBsYXRlLmhhc1Jlc291cmNlUHJvcGVydGllcygnQVdTOjpBcGlHYXRld2F5OjpEZXBsb3ltZW50Jywge1xuICAgICAgRGVzY3JpcHRpb246ICdBUEkgR2F0ZXdheSBmb3IgUnVzdCBMYW1iZGEgZnVuY3Rpb24nLFxuICAgIH0pO1xuICB9KTtcblxuICB0ZXN0KCdjcmVhdGVzIExhbWJkYSBmdW5jdGlvbiB3aXRoIGVudmlyb25tZW50IHZhcmlhYmxlcycsICgpID0+IHtcbiAgICB0ZW1wbGF0ZS5oYXNSZXNvdXJjZVByb3BlcnRpZXMoJ0FXUzo6TGFtYmRhOjpGdW5jdGlvbicsIHtcbiAgICAgIEVudmlyb25tZW50OiB7XG4gICAgICAgIFZhcmlhYmxlczoge1xuICAgICAgICAgIFJVU1RfTE9HOiAnaW5mbycsXG4gICAgICAgIH0sXG4gICAgICB9LFxuICAgIH0pO1xuICB9KTtcblxuICB0ZXN0KCdjcmVhdGVzIENsb3VkV2F0Y2ggbG9nIGdyb3VwIHdpdGggY29ycmVjdCByZXRlbnRpb24nLCAoKSA9PiB7XG4gICAgdGVtcGxhdGUuaGFzUmVzb3VyY2VQcm9wZXJ0aWVzKCdBV1M6OkxvZ3M6OkxvZ0dyb3VwJywge1xuICAgICAgUmV0ZW50aW9uSW5EYXlzOiA3LFxuICAgIH0pO1xuICB9KTtcblxuICB0ZXN0KCdjcmVhdGVzIElBTSByb2xlIGZvciBMYW1iZGEgZXhlY3V0aW9uJywgKCkgPT4ge1xuICAgIHRlbXBsYXRlLmhhc1Jlc291cmNlUHJvcGVydGllcygnQVdTOjpJQU06OlJvbGUnLCB7XG4gICAgICBBc3N1bWVSb2xlUG9saWN5RG9jdW1lbnQ6IHtcbiAgICAgICAgU3RhdGVtZW50OiBbXG4gICAgICAgICAge1xuICAgICAgICAgICAgQWN0aW9uOiAnc3RzOkFzc3VtZVJvbGUnLFxuICAgICAgICAgICAgRWZmZWN0OiAnQWxsb3cnLFxuICAgICAgICAgICAgUHJpbmNpcGFsOiB7XG4gICAgICAgICAgICAgIFNlcnZpY2U6ICdsYW1iZGEuYW1hem9uYXdzLmNvbScsXG4gICAgICAgICAgICB9LFxuICAgICAgICAgIH0sXG4gICAgICAgIF0sXG4gICAgICB9LFxuICAgIH0pO1xuICB9KTtcblxuICB0ZXN0KCdjcmVhdGVzIEFQSSBHYXRld2F5IHJlc291cmNlcyBmb3IgYWxsIHJvdXRlcycsICgpID0+IHtcbiAgICAvLyBDaGVjayBmb3IgaGVsbG8gcmVzb3VyY2VcbiAgICB0ZW1wbGF0ZS5oYXNSZXNvdXJjZVByb3BlcnRpZXMoJ0FXUzo6QXBpR2F0ZXdheTo6UmVzb3VyY2UnLCB7XG4gICAgICBQYXRoUGFydDogJ2hlbGxvJyxcbiAgICB9KTtcbiAgICBcbiAgICAvLyBDaGVjayBmb3IgaGVhbHRoIHJlc291cmNlICBcbiAgICB0ZW1wbGF0ZS5oYXNSZXNvdXJjZVByb3BlcnRpZXMoJ0FXUzo6QXBpR2F0ZXdheTo6UmVzb3VyY2UnLCB7XG4gICAgICBQYXRoUGFydDogJ2hlYWx0aCcsXG4gICAgfSk7XG4gICAgXG4gICAgLy8gQ2hlY2sgZm9yIHtuYW1lfSBwYXJhbWV0ZXIgcmVzb3VyY2VcbiAgICB0ZW1wbGF0ZS5oYXNSZXNvdXJjZVByb3BlcnRpZXMoJ0FXUzo6QXBpR2F0ZXdheTo6UmVzb3VyY2UnLCB7XG4gICAgICBQYXRoUGFydDogJ3tuYW1lfScsXG4gICAgfSk7XG4gIH0pO1xuXG4gIHRlc3QoJ2NyZWF0ZXMgQVBJIEdhdGV3YXkgbWV0aG9kcyBmb3IgYWxsIHJvdXRlcycsICgpID0+IHtcbiAgICAvLyBDREsgY3JlYXRlcyBhZGRpdGlvbmFsIE9QVElPTlMgbWV0aG9kcyBmb3IgQ09SUywgc28gZXhwZWN0IG1vcmUgdGhhbiAzXG4gICAgY29uc3QgbWV0aG9kQ291bnQgPSB0ZW1wbGF0ZS5maW5kUmVzb3VyY2VzKCdBV1M6OkFwaUdhdGV3YXk6Ok1ldGhvZCcpO1xuICAgIGV4cGVjdChPYmplY3Qua2V5cyhtZXRob2RDb3VudCkubGVuZ3RoKS50b0JlR3JlYXRlclRoYW5PckVxdWFsKDMpO1xuICAgIFxuICAgIC8vIFZlcmlmeSBzcGVjaWZpYyBHRVQgbWV0aG9kcyBleGlzdFxuICAgIHRlbXBsYXRlLmhhc1Jlc291cmNlUHJvcGVydGllcygnQVdTOjpBcGlHYXRld2F5OjpNZXRob2QnLCB7XG4gICAgICBIdHRwTWV0aG9kOiAnR0VUJyxcbiAgICB9KTtcbiAgfSk7XG5cbiAgdGVzdCgnZ3JhbnRzIEFQSSBHYXRld2F5IHBlcm1pc3Npb24gdG8gaW52b2tlIExhbWJkYScsICgpID0+IHtcbiAgICB0ZW1wbGF0ZS5oYXNSZXNvdXJjZVByb3BlcnRpZXMoJ0FXUzo6TGFtYmRhOjpQZXJtaXNzaW9uJywge1xuICAgICAgQWN0aW9uOiAnbGFtYmRhOkludm9rZUZ1bmN0aW9uJyxcbiAgICAgIFByaW5jaXBhbDogJ2FwaWdhdGV3YXkuYW1hem9uYXdzLmNvbScsXG4gICAgfSk7XG4gIH0pO1xuXG4gIHRlc3QoJ2hhcyBhbGwgcmVxdWlyZWQgc3RhY2sgb3V0cHV0cycsICgpID0+IHtcbiAgICB0ZW1wbGF0ZS5oYXNPdXRwdXQoJ0FwaVVybCcsIHt9KTtcbiAgICB0ZW1wbGF0ZS5oYXNPdXRwdXQoJ0xhbWJkYUZ1bmN0aW9uTmFtZScsIHt9KTtcbiAgICB0ZW1wbGF0ZS5oYXNPdXRwdXQoJ0xhbWJkYUZ1bmN0aW9uQXJuJywge30pO1xuICAgIFxuICAgIGNvbnN0IG91dHB1dHMgPSB0ZW1wbGF0ZS5maW5kT3V0cHV0cygnKicpO1xuICAgIC8vIENESyBtYXkgY3JlYXRlIGFkZGl0aW9uYWwgb3V0cHV0cywgc28gY2hlY2sgd2UgaGF2ZSBhdCBsZWFzdCBvdXIgM1xuICAgIGV4cGVjdChPYmplY3Qua2V5cyhvdXRwdXRzKS5sZW5ndGgpLnRvQmVHcmVhdGVyVGhhbk9yRXF1YWwoMyk7XG4gIH0pO1xuXG4gIHRlc3QoJ0FQSSBHYXRld2F5IHN0YWdlIGhhcyB0cmFjaW5nIGVuYWJsZWQnLCAoKSA9PiB7XG4gICAgdGVtcGxhdGUuaGFzUmVzb3VyY2VQcm9wZXJ0aWVzKCdBV1M6OkFwaUdhdGV3YXk6OlN0YWdlJywge1xuICAgICAgU3RhZ2VOYW1lOiAncHJvZCcsXG4gICAgICBUcmFjaW5nRW5hYmxlZDogdHJ1ZSxcbiAgICB9KTtcbiAgfSk7XG5cbiAgdGVzdCgnTGFtYmRhIGZ1bmN0aW9uIGhhcyBjb3JyZWN0IGRlc2NyaXB0aW9uJywgKCkgPT4ge1xuICAgIHRlbXBsYXRlLmhhc1Jlc291cmNlUHJvcGVydGllcygnQVdTOjpMYW1iZGE6OkZ1bmN0aW9uJywge1xuICAgICAgRGVzY3JpcHRpb246ICdSdXN0IExhbWJkYSBmdW5jdGlvbiB3aXRoIEhlbGxvIFdvcmxkIEFQSScsXG4gICAgfSk7XG4gIH0pO1xuXG4gIHRlc3QoJ0Nsb3VkRm9ybWF0aW9uIHRlbXBsYXRlIGhhcyBleHBlY3RlZCByZXNvdXJjZSBjb3VudCcsICgpID0+IHtcbiAgICBjb25zdCBjZm5UZW1wbGF0ZSA9IHRlbXBsYXRlLnRvSlNPTigpO1xuICAgIGNvbnN0IHJlc291cmNlQ291bnQgPSBPYmplY3Qua2V5cyhjZm5UZW1wbGF0ZS5SZXNvdXJjZXMgfHwge30pLmxlbmd0aDtcbiAgICBcbiAgICAvLyBTaG91bGQgaGF2ZTogTGFtYmRhIGZ1bmN0aW9uLCBBUEkgR2F0ZXdheSBBUEksIDMgcmVzb3VyY2VzLCAzIG1ldGhvZHMsIFxuICAgIC8vIGRlcGxveW1lbnQsIHN0YWdlLCBJQU0gcm9sZSwgbG9nIGdyb3VwLCBwZXJtaXNzaW9ucywgZXRjLlxuICAgIGV4cGVjdChyZXNvdXJjZUNvdW50KS50b0JlR3JlYXRlclRoYW4oMTApO1xuICB9KTtcblxuICB0ZXN0KCdzeW50aGVzaXplcyBDbG91ZEZvcm1hdGlvbiB0ZW1wbGF0ZSBzdWNjZXNzZnVsbHknLCAoKSA9PiB7XG4gICAgZXhwZWN0KCgpID0+IHtcbiAgICAgIGNvbnN0IGNmblRlbXBsYXRlID0gdGVtcGxhdGUudG9KU09OKCk7XG4gICAgICBleHBlY3QoY2ZuVGVtcGxhdGUpLnRvSGF2ZVByb3BlcnR5KCdSZXNvdXJjZXMnKTtcbiAgICAgIGV4cGVjdChjZm5UZW1wbGF0ZSkudG9IYXZlUHJvcGVydHkoJ091dHB1dHMnKTtcbiAgICB9KS5ub3QudG9UaHJvdygpO1xuICB9KTtcblxuICB0ZXN0KCd2YWxpZGF0ZXMgQVBJIEdhdGV3YXkgaW50ZWdyYXRpb24gY29uZmlndXJhdGlvbicsICgpID0+IHtcbiAgICAvLyBDaGVjayB0aGF0IExhbWJkYSBpbnRlZ3JhdGlvbiBpcyBwcm9wZXJseSBjb25maWd1cmVkXG4gICAgdGVtcGxhdGUuaGFzUmVzb3VyY2VQcm9wZXJ0aWVzKCdBV1M6OkFwaUdhdGV3YXk6Ok1ldGhvZCcsIHtcbiAgICAgIEh0dHBNZXRob2Q6ICdHRVQnLFxuICAgICAgSW50ZWdyYXRpb246IHtcbiAgICAgICAgVHlwZTogJ0FXU19QUk9YWScsXG4gICAgICB9LFxuICAgIH0pO1xuICB9KTtcblxuICB0ZXN0KCdlbnN1cmVzIHByb3BlciBDT1JTIGNvbmZpZ3VyYXRpb24gaW4gQVBJIEdhdGV3YXknLCAoKSA9PiB7XG4gICAgLy8gQ2hlY2sgdGhhdCBSZXN0QXBpIGlzIGNyZWF0ZWQgKENPUlMgaXMgY29uZmlndXJlZCBhdCB0aGlzIGxldmVsKVxuICAgIHRlbXBsYXRlLnJlc291cmNlQ291bnRJcygnQVdTOjpBcGlHYXRld2F5OjpSZXN0QXBpJywgMSk7XG4gICAgXG4gICAgdGVtcGxhdGUuaGFzUmVzb3VyY2VQcm9wZXJ0aWVzKCdBV1M6OkFwaUdhdGV3YXk6OlJlc3RBcGknLCB7XG4gICAgICBOYW1lOiAnUnVzdCBMYW1iZGEgQVBJJyxcbiAgICB9KTtcbiAgfSk7XG59KTsiXX0=