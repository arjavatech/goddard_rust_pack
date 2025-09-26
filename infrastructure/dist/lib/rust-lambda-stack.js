"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.RustLambdaStack = void 0;
const cdk = require("aws-cdk-lib");
const lambda = require("aws-cdk-lib/aws-lambda");
const apigateway = require("aws-cdk-lib/aws-apigateway");
const logs = require("aws-cdk-lib/aws-logs");
const path = require("path");
class RustLambdaStack extends cdk.Stack {
    constructor(scope, id, props) {
        super(scope, id, props);
        // Lambda function for Rust code
        // Using ARM64 architecture for up to 34% better price performance and 19% better performance
        // See: https://aws.amazon.com/blogs/compute/migrating-aws-lambda-functions-to-arm-based-aws-graviton2-processors/
        const rustLambda = new lambda.Function(this, 'GoddardLambda', {
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
            logGroup: new logs.LogGroup(this, 'GoddardLambdaLogGroup', {
                retention: logs.RetentionDays.ONE_WEEK,
                removalPolicy: cdk.RemovalPolicy.DESTROY,
            }),
            description: 'Goddard Backend Lambda function with API endpoints',
        });
        // API Gateway
        const api = new apigateway.RestApi(this, 'GoddardApi', {
            restApiName: 'Goddard Backend API',
            description: 'API Gateway for Goddard Backend Lambda function',
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
            description: 'API Gateway URL',
            exportName: 'GoddardApiUrl',
        });
        new cdk.CfnOutput(this, 'LambdaFunctionName', {
            value: rustLambda.functionName,
            description: 'Lambda Function Name',
            exportName: 'GoddardLambdaFunctionName',
        });
        new cdk.CfnOutput(this, 'LambdaFunctionArn', {
            value: rustLambda.functionArn,
            description: 'Lambda Function ARN',
            exportName: 'GoddardLambdaFunctionArn',
        });
    }
}
exports.RustLambdaStack = RustLambdaStack;
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoicnVzdC1sYW1iZGEtc3RhY2suanMiLCJzb3VyY2VSb290IjoiIiwic291cmNlcyI6WyIuLi8uLi9saWIvcnVzdC1sYW1iZGEtc3RhY2sudHMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6Ijs7O0FBQUEsbUNBQW1DO0FBQ25DLGlEQUFpRDtBQUNqRCx5REFBeUQ7QUFDekQsNkNBQTZDO0FBQzdDLDZCQUE2QjtBQUc3QixNQUFhLGVBQWdCLFNBQVEsR0FBRyxDQUFDLEtBQUs7SUFDNUMsWUFBWSxLQUFnQixFQUFFLEVBQVUsRUFBRSxLQUFzQjtRQUM5RCxLQUFLLENBQUMsS0FBSyxFQUFFLEVBQUUsRUFBRSxLQUFLLENBQUMsQ0FBQztRQUV4QixnQ0FBZ0M7UUFDaEMsNkZBQTZGO1FBQzdGLGtIQUFrSDtRQUNsSCxNQUFNLFVBQVUsR0FBRyxJQUFJLE1BQU0sQ0FBQyxRQUFRLENBQUMsSUFBSSxFQUFFLGVBQWUsRUFBRTtZQUM1RCxPQUFPLEVBQUUsTUFBTSxDQUFDLE9BQU8sQ0FBQyxlQUFlLEVBQUUsbUNBQW1DO1lBQzVFLFlBQVksRUFBRSxNQUFNLENBQUMsWUFBWSxDQUFDLE1BQU0sRUFBRSxrQ0FBa0M7WUFDNUUsT0FBTyxFQUFFLFdBQVc7WUFDcEIsSUFBSSxFQUFFLE1BQU0sQ0FBQyxJQUFJLENBQUMsU0FBUyxDQUFDLElBQUksQ0FBQyxJQUFJLENBQUMsU0FBUyxFQUFFLG9EQUFvRCxDQUFDLEVBQUU7Z0JBQ3RHLE9BQU8sRUFBRSxDQUFDLElBQUksRUFBRSxZQUFZLENBQUM7YUFDOUIsQ0FBQztZQUNGLFVBQVUsRUFBRSxHQUFHO1lBQ2YsT0FBTyxFQUFFLEdBQUcsQ0FBQyxRQUFRLENBQUMsT0FBTyxDQUFDLEVBQUUsQ0FBQztZQUNqQyxXQUFXLEVBQUU7Z0JBQ1gsUUFBUSxFQUFFLE1BQU07YUFDakI7WUFDRCxRQUFRLEVBQUUsSUFBSSxJQUFJLENBQUMsUUFBUSxDQUFDLElBQUksRUFBRSx1QkFBdUIsRUFBRTtnQkFDekQsU0FBUyxFQUFFLElBQUksQ0FBQyxhQUFhLENBQUMsUUFBUTtnQkFDdEMsYUFBYSxFQUFFLEdBQUcsQ0FBQyxhQUFhLENBQUMsT0FBTzthQUN6QyxDQUFDO1lBQ0YsV0FBVyxFQUFFLG9EQUFvRDtTQUNsRSxDQUFDLENBQUM7UUFFSCxjQUFjO1FBQ2QsTUFBTSxHQUFHLEdBQUcsSUFBSSxVQUFVLENBQUMsT0FBTyxDQUFDLElBQUksRUFBRSxZQUFZLEVBQUU7WUFDckQsV0FBVyxFQUFFLHFCQUFxQjtZQUNsQyxXQUFXLEVBQUUsaURBQWlEO1lBQzlELGFBQWEsRUFBRTtnQkFDYixTQUFTLEVBQUUsTUFBTTtnQkFDakIsY0FBYyxFQUFFLElBQUk7Z0JBQ3BCLGNBQWMsRUFBRSxJQUFJO2FBQ3JCO1lBQ0QsMkJBQTJCLEVBQUU7Z0JBQzNCLFlBQVksRUFBRSxVQUFVLENBQUMsSUFBSSxDQUFDLFdBQVc7Z0JBQ3pDLFlBQVksRUFBRSxVQUFVLENBQUMsSUFBSSxDQUFDLFdBQVc7Z0JBQ3pDLFlBQVksRUFBRSxDQUFDLGNBQWMsRUFBRSxlQUFlLEVBQUUsY0FBYyxFQUFFLGFBQWEsRUFBRSxXQUFXLENBQUM7YUFDNUY7U0FDRixDQUFDLENBQUM7UUFFSCxnQ0FBZ0M7UUFDaEMsTUFBTSxpQkFBaUIsR0FBRyxJQUFJLFVBQVUsQ0FBQyxpQkFBaUIsQ0FBQyxVQUFVLEVBQUU7WUFDckUsS0FBSyxFQUFFLElBQUk7U0FDWixDQUFDLENBQUM7UUFFSCxtQkFBbUI7UUFDbkIsR0FBRyxDQUFDLElBQUksQ0FBQyxTQUFTLENBQUMsS0FBSyxFQUFFLGlCQUFpQixDQUFDLENBQUM7UUFFN0MsNENBQTRDO1FBQzVDLE1BQU0sYUFBYSxHQUFHLEdBQUcsQ0FBQyxJQUFJLENBQUMsV0FBVyxDQUFDLFVBQVUsQ0FBQyxDQUFDO1FBQ3ZELGFBQWEsQ0FBQyxTQUFTLENBQUMsS0FBSyxFQUFFLGlCQUFpQixDQUFDLENBQUM7UUFFbEQsVUFBVTtRQUNWLElBQUksR0FBRyxDQUFDLFNBQVMsQ0FBQyxJQUFJLEVBQUUsUUFBUSxFQUFFO1lBQ2hDLEtBQUssRUFBRSxHQUFHLENBQUMsR0FBRztZQUNkLFdBQVcsRUFBRSxpQkFBaUI7WUFDOUIsVUFBVSxFQUFFLGVBQWU7U0FDNUIsQ0FBQyxDQUFDO1FBRUgsSUFBSSxHQUFHLENBQUMsU0FBUyxDQUFDLElBQUksRUFBRSxvQkFBb0IsRUFBRTtZQUM1QyxLQUFLLEVBQUUsVUFBVSxDQUFDLFlBQVk7WUFDOUIsV0FBVyxFQUFFLHNCQUFzQjtZQUNuQyxVQUFVLEVBQUUsMkJBQTJCO1NBQ3hDLENBQUMsQ0FBQztRQUVILElBQUksR0FBRyxDQUFDLFNBQVMsQ0FBQyxJQUFJLEVBQUUsbUJBQW1CLEVBQUU7WUFDM0MsS0FBSyxFQUFFLFVBQVUsQ0FBQyxXQUFXO1lBQzdCLFdBQVcsRUFBRSxxQkFBcUI7WUFDbEMsVUFBVSxFQUFFLDBCQUEwQjtTQUN2QyxDQUFDLENBQUM7SUFDTCxDQUFDO0NBQ0Y7QUF6RUQsMENBeUVDIiwic291cmNlc0NvbnRlbnQiOlsiaW1wb3J0ICogYXMgY2RrIGZyb20gJ2F3cy1jZGstbGliJztcbmltcG9ydCAqIGFzIGxhbWJkYSBmcm9tICdhd3MtY2RrLWxpYi9hd3MtbGFtYmRhJztcbmltcG9ydCAqIGFzIGFwaWdhdGV3YXkgZnJvbSAnYXdzLWNkay1saWIvYXdzLWFwaWdhdGV3YXknO1xuaW1wb3J0ICogYXMgbG9ncyBmcm9tICdhd3MtY2RrLWxpYi9hd3MtbG9ncyc7XG5pbXBvcnQgKiBhcyBwYXRoIGZyb20gJ3BhdGgnO1xuaW1wb3J0IHsgQ29uc3RydWN0IH0gZnJvbSAnY29uc3RydWN0cyc7XG5cbmV4cG9ydCBjbGFzcyBSdXN0TGFtYmRhU3RhY2sgZXh0ZW5kcyBjZGsuU3RhY2sge1xuICBjb25zdHJ1Y3RvcihzY29wZTogQ29uc3RydWN0LCBpZDogc3RyaW5nLCBwcm9wcz86IGNkay5TdGFja1Byb3BzKSB7XG4gICAgc3VwZXIoc2NvcGUsIGlkLCBwcm9wcyk7XG5cbiAgICAvLyBMYW1iZGEgZnVuY3Rpb24gZm9yIFJ1c3QgY29kZVxuICAgIC8vIFVzaW5nIEFSTTY0IGFyY2hpdGVjdHVyZSBmb3IgdXAgdG8gMzQlIGJldHRlciBwcmljZSBwZXJmb3JtYW5jZSBhbmQgMTklIGJldHRlciBwZXJmb3JtYW5jZVxuICAgIC8vIFNlZTogaHR0cHM6Ly9hd3MuYW1hem9uLmNvbS9ibG9ncy9jb21wdXRlL21pZ3JhdGluZy1hd3MtbGFtYmRhLWZ1bmN0aW9ucy10by1hcm0tYmFzZWQtYXdzLWdyYXZpdG9uMi1wcm9jZXNzb3JzL1xuICAgIGNvbnN0IHJ1c3RMYW1iZGEgPSBuZXcgbGFtYmRhLkZ1bmN0aW9uKHRoaXMsICdHb2RkYXJkTGFtYmRhJywge1xuICAgICAgcnVudGltZTogbGFtYmRhLlJ1bnRpbWUuUFJPVklERURfQUwyMDIzLCAvLyBBbWF6b24gTGludXggMjAyMyBzdXBwb3J0cyBBUk02NFxuICAgICAgYXJjaGl0ZWN0dXJlOiBsYW1iZGEuQXJjaGl0ZWN0dXJlLkFSTV82NCwgLy8gQVdTIEdyYXZpdG9uMiBwcm9jZXNzb3IgKEFSTTY0KVxuICAgICAgaGFuZGxlcjogJ2Jvb3RzdHJhcCcsXG4gICAgICBjb2RlOiBsYW1iZGEuQ29kZS5mcm9tQXNzZXQocGF0aC5qb2luKF9fZGlybmFtZSwgJy4uLy4uL2xhbWJkYS9nb2RkYXJkL3RhcmdldC9sYW1iZGEvZ29kZGFyZC1iYWNrZW5kJyksIHtcbiAgICAgICAgZXhjbHVkZTogWycqKicsICchYm9vdHN0cmFwJ10sXG4gICAgICB9KSxcbiAgICAgIG1lbW9yeVNpemU6IDI1NixcbiAgICAgIHRpbWVvdXQ6IGNkay5EdXJhdGlvbi5zZWNvbmRzKDMwKSxcbiAgICAgIGVudmlyb25tZW50OiB7XG4gICAgICAgIFJVU1RfTE9HOiAnaW5mbycsXG4gICAgICB9LFxuICAgICAgbG9nR3JvdXA6IG5ldyBsb2dzLkxvZ0dyb3VwKHRoaXMsICdHb2RkYXJkTGFtYmRhTG9nR3JvdXAnLCB7XG4gICAgICAgIHJldGVudGlvbjogbG9ncy5SZXRlbnRpb25EYXlzLk9ORV9XRUVLLFxuICAgICAgICByZW1vdmFsUG9saWN5OiBjZGsuUmVtb3ZhbFBvbGljeS5ERVNUUk9ZLFxuICAgICAgfSksXG4gICAgICBkZXNjcmlwdGlvbjogJ0dvZGRhcmQgQmFja2VuZCBMYW1iZGEgZnVuY3Rpb24gd2l0aCBBUEkgZW5kcG9pbnRzJyxcbiAgICB9KTtcblxuICAgIC8vIEFQSSBHYXRld2F5XG4gICAgY29uc3QgYXBpID0gbmV3IGFwaWdhdGV3YXkuUmVzdEFwaSh0aGlzLCAnR29kZGFyZEFwaScsIHtcbiAgICAgIHJlc3RBcGlOYW1lOiAnR29kZGFyZCBCYWNrZW5kIEFQSScsXG4gICAgICBkZXNjcmlwdGlvbjogJ0FQSSBHYXRld2F5IGZvciBHb2RkYXJkIEJhY2tlbmQgTGFtYmRhIGZ1bmN0aW9uJyxcbiAgICAgIGRlcGxveU9wdGlvbnM6IHtcbiAgICAgICAgc3RhZ2VOYW1lOiAncHJvZCcsXG4gICAgICAgIHRyYWNpbmdFbmFibGVkOiB0cnVlLFxuICAgICAgICBtZXRyaWNzRW5hYmxlZDogdHJ1ZSxcbiAgICAgIH0sXG4gICAgICBkZWZhdWx0Q29yc1ByZWZsaWdodE9wdGlvbnM6IHtcbiAgICAgICAgYWxsb3dPcmlnaW5zOiBhcGlnYXRld2F5LkNvcnMuQUxMX09SSUdJTlMsXG4gICAgICAgIGFsbG93TWV0aG9kczogYXBpZ2F0ZXdheS5Db3JzLkFMTF9NRVRIT0RTLFxuICAgICAgICBhbGxvd0hlYWRlcnM6IFsnQ29udGVudC1UeXBlJywgJ0F1dGhvcml6YXRpb24nLCAneC1yZXF1ZXN0LWlkJywgJ3gtc2Nob29sLWlkJywgJ3gtYXBpLWtleSddLFxuICAgICAgfSxcbiAgICB9KTtcblxuICAgIC8vIExhbWJkYSBpbnRlZ3JhdGlvbiB3aXRoIHByb3h5XG4gICAgY29uc3QgbGFtYmRhSW50ZWdyYXRpb24gPSBuZXcgYXBpZ2F0ZXdheS5MYW1iZGFJbnRlZ3JhdGlvbihydXN0TGFtYmRhLCB7XG4gICAgICBwcm94eTogdHJ1ZSxcbiAgICB9KTtcblxuICAgIC8vIEhhbmRsZSByb290IHBhdGhcbiAgICBhcGkucm9vdC5hZGRNZXRob2QoJ0FOWScsIGxhbWJkYUludGVncmF0aW9uKTtcbiAgICBcbiAgICAvLyBDcmVhdGUgcHJveHkgcmVzb3VyY2UgZm9yIGFsbCBvdGhlciBwYXRoc1xuICAgIGNvbnN0IHByb3h5UmVzb3VyY2UgPSBhcGkucm9vdC5hZGRSZXNvdXJjZSgne3Byb3h5K30nKTtcbiAgICBwcm94eVJlc291cmNlLmFkZE1ldGhvZCgnQU5ZJywgbGFtYmRhSW50ZWdyYXRpb24pO1xuXG4gICAgLy8gT3V0cHV0c1xuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdBcGlVcmwnLCB7XG4gICAgICB2YWx1ZTogYXBpLnVybCxcbiAgICAgIGRlc2NyaXB0aW9uOiAnQVBJIEdhdGV3YXkgVVJMJyxcbiAgICAgIGV4cG9ydE5hbWU6ICdHb2RkYXJkQXBpVXJsJyxcbiAgICB9KTtcblxuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdMYW1iZGFGdW5jdGlvbk5hbWUnLCB7XG4gICAgICB2YWx1ZTogcnVzdExhbWJkYS5mdW5jdGlvbk5hbWUsXG4gICAgICBkZXNjcmlwdGlvbjogJ0xhbWJkYSBGdW5jdGlvbiBOYW1lJyxcbiAgICAgIGV4cG9ydE5hbWU6ICdHb2RkYXJkTGFtYmRhRnVuY3Rpb25OYW1lJyxcbiAgICB9KTtcblxuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdMYW1iZGFGdW5jdGlvbkFybicsIHtcbiAgICAgIHZhbHVlOiBydXN0TGFtYmRhLmZ1bmN0aW9uQXJuLFxuICAgICAgZGVzY3JpcHRpb246ICdMYW1iZGEgRnVuY3Rpb24gQVJOJyxcbiAgICAgIGV4cG9ydE5hbWU6ICdHb2RkYXJkTGFtYmRhRnVuY3Rpb25Bcm4nLFxuICAgIH0pO1xuICB9XG59Il19