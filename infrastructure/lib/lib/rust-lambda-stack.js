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
        // Handle root path
        api.root.addMethod('ANY', lambdaIntegration);
        // Create proxy resource for all other paths
        const proxyResource = api.root.addResource('{proxy+}');
        proxyResource.addMethod('ANY', lambdaIntegration);
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
exports.RustLambdaStack = RustLambdaStack;
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoicnVzdC1sYW1iZGEtc3RhY2suanMiLCJzb3VyY2VSb290IjoiIiwic291cmNlcyI6WyIuLi9ydXN0LWxhbWJkYS1zdGFjay50cyJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiOzs7QUFBQSxtQ0FBbUM7QUFDbkMsaURBQWlEO0FBQ2pELHlEQUF5RDtBQUN6RCw2Q0FBNkM7QUFDN0MsNkJBQTZCO0FBRzdCLE1BQWEsZUFBZ0IsU0FBUSxHQUFHLENBQUMsS0FBSztJQUM1QyxZQUFZLEtBQWdCLEVBQUUsRUFBVSxFQUFFLEtBQXNCO1FBQzlELEtBQUssQ0FBQyxLQUFLLEVBQUUsRUFBRSxFQUFFLEtBQUssQ0FBQyxDQUFDO1FBRXhCLGdDQUFnQztRQUNoQyxNQUFNLFVBQVUsR0FBRyxJQUFJLE1BQU0sQ0FBQyxRQUFRLENBQUMsSUFBSSxFQUFFLHNCQUFzQixFQUFFO1lBQ25FLE9BQU8sRUFBRSxNQUFNLENBQUMsT0FBTyxDQUFDLGVBQWU7WUFDdkMsWUFBWSxFQUFFLE1BQU0sQ0FBQyxZQUFZLENBQUMsTUFBTTtZQUN4QyxPQUFPLEVBQUUsV0FBVztZQUNwQixJQUFJLEVBQUUsTUFBTSxDQUFDLElBQUksQ0FBQyxTQUFTLENBQUMsSUFBSSxDQUFDLElBQUksQ0FBQyxTQUFTLEVBQUUsb0RBQW9ELENBQUMsRUFBRTtnQkFDdEcsT0FBTyxFQUFFLENBQUMsSUFBSSxFQUFFLFlBQVksQ0FBQzthQUM5QixDQUFDO1lBQ0YsVUFBVSxFQUFFLEdBQUc7WUFDZixPQUFPLEVBQUUsR0FBRyxDQUFDLFFBQVEsQ0FBQyxPQUFPLENBQUMsRUFBRSxDQUFDO1lBQ2pDLFdBQVcsRUFBRTtnQkFDWCxRQUFRLEVBQUUsTUFBTTthQUNqQjtZQUNELFFBQVEsRUFBRSxJQUFJLElBQUksQ0FBQyxRQUFRLENBQUMsSUFBSSxFQUFFLG9CQUFvQixFQUFFO2dCQUN0RCxTQUFTLEVBQUUsSUFBSSxDQUFDLGFBQWEsQ0FBQyxRQUFRO2dCQUN0QyxhQUFhLEVBQUUsR0FBRyxDQUFDLGFBQWEsQ0FBQyxPQUFPO2FBQ3pDLENBQUM7WUFDRixXQUFXLEVBQUUsMkNBQTJDO1NBQ3pELENBQUMsQ0FBQztRQUVILGNBQWM7UUFDZCxNQUFNLEdBQUcsR0FBRyxJQUFJLFVBQVUsQ0FBQyxPQUFPLENBQUMsSUFBSSxFQUFFLGVBQWUsRUFBRTtZQUN4RCxXQUFXLEVBQUUsaUJBQWlCO1lBQzlCLFdBQVcsRUFBRSxzQ0FBc0M7WUFDbkQsYUFBYSxFQUFFO2dCQUNiLFNBQVMsRUFBRSxNQUFNO2dCQUNqQixjQUFjLEVBQUUsSUFBSTtnQkFDcEIsY0FBYyxFQUFFLElBQUk7YUFDckI7WUFDRCwyQkFBMkIsRUFBRTtnQkFDM0IsWUFBWSxFQUFFLFVBQVUsQ0FBQyxJQUFJLENBQUMsV0FBVztnQkFDekMsWUFBWSxFQUFFLFVBQVUsQ0FBQyxJQUFJLENBQUMsV0FBVztnQkFDekMsWUFBWSxFQUFFLENBQUMsY0FBYyxFQUFFLGVBQWUsQ0FBQzthQUNoRDtTQUNGLENBQUMsQ0FBQztRQUVILGdDQUFnQztRQUNoQyxNQUFNLGlCQUFpQixHQUFHLElBQUksVUFBVSxDQUFDLGlCQUFpQixDQUFDLFVBQVUsRUFBRTtZQUNyRSxLQUFLLEVBQUUsSUFBSTtTQUNaLENBQUMsQ0FBQztRQUVILG1CQUFtQjtRQUNuQixHQUFHLENBQUMsSUFBSSxDQUFDLFNBQVMsQ0FBQyxLQUFLLEVBQUUsaUJBQWlCLENBQUMsQ0FBQztRQUU3Qyw0Q0FBNEM7UUFDNUMsTUFBTSxhQUFhLEdBQUcsR0FBRyxDQUFDLElBQUksQ0FBQyxXQUFXLENBQUMsVUFBVSxDQUFDLENBQUM7UUFDdkQsYUFBYSxDQUFDLFNBQVMsQ0FBQyxLQUFLLEVBQUUsaUJBQWlCLENBQUMsQ0FBQztRQUVsRCxVQUFVO1FBQ1YsSUFBSSxHQUFHLENBQUMsU0FBUyxDQUFDLElBQUksRUFBRSxRQUFRLEVBQUU7WUFDaEMsS0FBSyxFQUFFLEdBQUcsQ0FBQyxHQUFHO1lBQ2QsV0FBVyxFQUFFLGlCQUFpQjtZQUM5QixVQUFVLEVBQUUsa0JBQWtCO1NBQy9CLENBQUMsQ0FBQztRQUVILElBQUksR0FBRyxDQUFDLFNBQVMsQ0FBQyxJQUFJLEVBQUUsb0JBQW9CLEVBQUU7WUFDNUMsS0FBSyxFQUFFLFVBQVUsQ0FBQyxZQUFZO1lBQzlCLFdBQVcsRUFBRSxzQkFBc0I7WUFDbkMsVUFBVSxFQUFFLHdCQUF3QjtTQUNyQyxDQUFDLENBQUM7UUFFSCxJQUFJLEdBQUcsQ0FBQyxTQUFTLENBQUMsSUFBSSxFQUFFLG1CQUFtQixFQUFFO1lBQzNDLEtBQUssRUFBRSxVQUFVLENBQUMsV0FBVztZQUM3QixXQUFXLEVBQUUscUJBQXFCO1lBQ2xDLFVBQVUsRUFBRSx1QkFBdUI7U0FDcEMsQ0FBQyxDQUFDO0lBQ0wsQ0FBQztDQUNGO0FBdkVELDBDQXVFQyIsInNvdXJjZXNDb250ZW50IjpbImltcG9ydCAqIGFzIGNkayBmcm9tICdhd3MtY2RrLWxpYic7XG5pbXBvcnQgKiBhcyBsYW1iZGEgZnJvbSAnYXdzLWNkay1saWIvYXdzLWxhbWJkYSc7XG5pbXBvcnQgKiBhcyBhcGlnYXRld2F5IGZyb20gJ2F3cy1jZGstbGliL2F3cy1hcGlnYXRld2F5JztcbmltcG9ydCAqIGFzIGxvZ3MgZnJvbSAnYXdzLWNkay1saWIvYXdzLWxvZ3MnO1xuaW1wb3J0ICogYXMgcGF0aCBmcm9tICdwYXRoJztcbmltcG9ydCB7IENvbnN0cnVjdCB9IGZyb20gJ2NvbnN0cnVjdHMnO1xuXG5leHBvcnQgY2xhc3MgUnVzdExhbWJkYVN0YWNrIGV4dGVuZHMgY2RrLlN0YWNrIHtcbiAgY29uc3RydWN0b3Ioc2NvcGU6IENvbnN0cnVjdCwgaWQ6IHN0cmluZywgcHJvcHM/OiBjZGsuU3RhY2tQcm9wcykge1xuICAgIHN1cGVyKHNjb3BlLCBpZCwgcHJvcHMpO1xuXG4gICAgLy8gTGFtYmRhIGZ1bmN0aW9uIGZvciBSdXN0IGNvZGVcbiAgICBjb25zdCBydXN0TGFtYmRhID0gbmV3IGxhbWJkYS5GdW5jdGlvbih0aGlzLCAnUnVzdEhlbGxvV29ybGRMYW1iZGEnLCB7XG4gICAgICBydW50aW1lOiBsYW1iZGEuUnVudGltZS5QUk9WSURFRF9BTDIwMjMsXG4gICAgICBhcmNoaXRlY3R1cmU6IGxhbWJkYS5BcmNoaXRlY3R1cmUuQVJNXzY0LFxuICAgICAgaGFuZGxlcjogJ2Jvb3RzdHJhcCcsXG4gICAgICBjb2RlOiBsYW1iZGEuQ29kZS5mcm9tQXNzZXQocGF0aC5qb2luKF9fZGlybmFtZSwgJy4uLy4uL2xhbWJkYS9oZWxsby13b3JsZC90YXJnZXQvbGFtYmRhL2hlbGxvLXdvcmxkJyksIHtcbiAgICAgICAgZXhjbHVkZTogWycqKicsICchYm9vdHN0cmFwJ10sXG4gICAgICB9KSxcbiAgICAgIG1lbW9yeVNpemU6IDI1NixcbiAgICAgIHRpbWVvdXQ6IGNkay5EdXJhdGlvbi5zZWNvbmRzKDMwKSxcbiAgICAgIGVudmlyb25tZW50OiB7XG4gICAgICAgIFJVU1RfTE9HOiAnaW5mbycsXG4gICAgICB9LFxuICAgICAgbG9nR3JvdXA6IG5ldyBsb2dzLkxvZ0dyb3VwKHRoaXMsICdSdXN0TGFtYmRhTG9nR3JvdXAnLCB7XG4gICAgICAgIHJldGVudGlvbjogbG9ncy5SZXRlbnRpb25EYXlzLk9ORV9XRUVLLFxuICAgICAgICByZW1vdmFsUG9saWN5OiBjZGsuUmVtb3ZhbFBvbGljeS5ERVNUUk9ZLFxuICAgICAgfSksXG4gICAgICBkZXNjcmlwdGlvbjogJ1J1c3QgTGFtYmRhIGZ1bmN0aW9uIHdpdGggSGVsbG8gV29ybGQgQVBJJyxcbiAgICB9KTtcblxuICAgIC8vIEFQSSBHYXRld2F5XG4gICAgY29uc3QgYXBpID0gbmV3IGFwaWdhdGV3YXkuUmVzdEFwaSh0aGlzLCAnUnVzdExhbWJkYUFwaScsIHtcbiAgICAgIHJlc3RBcGlOYW1lOiAnUnVzdCBMYW1iZGEgQVBJJyxcbiAgICAgIGRlc2NyaXB0aW9uOiAnQVBJIEdhdGV3YXkgZm9yIFJ1c3QgTGFtYmRhIGZ1bmN0aW9uJyxcbiAgICAgIGRlcGxveU9wdGlvbnM6IHtcbiAgICAgICAgc3RhZ2VOYW1lOiAncHJvZCcsXG4gICAgICAgIHRyYWNpbmdFbmFibGVkOiB0cnVlLFxuICAgICAgICBtZXRyaWNzRW5hYmxlZDogdHJ1ZSxcbiAgICAgIH0sXG4gICAgICBkZWZhdWx0Q29yc1ByZWZsaWdodE9wdGlvbnM6IHtcbiAgICAgICAgYWxsb3dPcmlnaW5zOiBhcGlnYXRld2F5LkNvcnMuQUxMX09SSUdJTlMsXG4gICAgICAgIGFsbG93TWV0aG9kczogYXBpZ2F0ZXdheS5Db3JzLkFMTF9NRVRIT0RTLFxuICAgICAgICBhbGxvd0hlYWRlcnM6IFsnQ29udGVudC1UeXBlJywgJ0F1dGhvcml6YXRpb24nXSxcbiAgICAgIH0sXG4gICAgfSk7XG5cbiAgICAvLyBMYW1iZGEgaW50ZWdyYXRpb24gd2l0aCBwcm94eVxuICAgIGNvbnN0IGxhbWJkYUludGVncmF0aW9uID0gbmV3IGFwaWdhdGV3YXkuTGFtYmRhSW50ZWdyYXRpb24ocnVzdExhbWJkYSwge1xuICAgICAgcHJveHk6IHRydWUsXG4gICAgfSk7XG5cbiAgICAvLyBIYW5kbGUgcm9vdCBwYXRoXG4gICAgYXBpLnJvb3QuYWRkTWV0aG9kKCdBTlknLCBsYW1iZGFJbnRlZ3JhdGlvbik7XG4gICAgXG4gICAgLy8gQ3JlYXRlIHByb3h5IHJlc291cmNlIGZvciBhbGwgb3RoZXIgcGF0aHNcbiAgICBjb25zdCBwcm94eVJlc291cmNlID0gYXBpLnJvb3QuYWRkUmVzb3VyY2UoJ3twcm94eSt9Jyk7XG4gICAgcHJveHlSZXNvdXJjZS5hZGRNZXRob2QoJ0FOWScsIGxhbWJkYUludGVncmF0aW9uKTtcblxuICAgIC8vIE91dHB1dHNcbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnQXBpVXJsJywge1xuICAgICAgdmFsdWU6IGFwaS51cmwsXG4gICAgICBkZXNjcmlwdGlvbjogJ0FQSSBHYXRld2F5IFVSTCcsXG4gICAgICBleHBvcnROYW1lOiAnUnVzdExhbWJkYUFwaVVybCcsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnTGFtYmRhRnVuY3Rpb25OYW1lJywge1xuICAgICAgdmFsdWU6IHJ1c3RMYW1iZGEuZnVuY3Rpb25OYW1lLFxuICAgICAgZGVzY3JpcHRpb246ICdMYW1iZGEgRnVuY3Rpb24gTmFtZScsXG4gICAgICBleHBvcnROYW1lOiAnUnVzdExhbWJkYUZ1bmN0aW9uTmFtZScsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnTGFtYmRhRnVuY3Rpb25Bcm4nLCB7XG4gICAgICB2YWx1ZTogcnVzdExhbWJkYS5mdW5jdGlvbkFybixcbiAgICAgIGRlc2NyaXB0aW9uOiAnTGFtYmRhIEZ1bmN0aW9uIEFSTicsXG4gICAgICBleHBvcnROYW1lOiAnUnVzdExhbWJkYUZ1bmN0aW9uQXJuJyxcbiAgICB9KTtcbiAgfVxufSJdfQ==