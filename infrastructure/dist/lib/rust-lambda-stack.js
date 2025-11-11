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
        const rustLambda = new lambda.Function(this, 'GoddardProdLambda', {
            functionName: 'goddard-prod',
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
            logGroup: new logs.LogGroup(this, 'GoddardProdLambdaLogGroup', {
                logGroupName: '/aws/lambda/goddard-prod',
                retention: logs.RetentionDays.ONE_WEEK,
                removalPolicy: cdk.RemovalPolicy.DESTROY,
            }),
            description: 'Goddard Production - Backend Lambda function with API endpoints',
        });
        // API Gateway
        const api = new apigateway.RestApi(this, 'GoddardProdApi', {
            restApiName: 'Goddard Production API',
            description: 'Production API Gateway for Goddard Backend Lambda function',
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
            description: 'Production API Gateway URL',
            exportName: 'GoddardProdApiUrl',
        });
        new cdk.CfnOutput(this, 'LambdaFunctionName', {
            value: rustLambda.functionName,
            description: 'Production Lambda Function Name',
            exportName: 'GoddardProdLambdaFunctionName',
        });
        new cdk.CfnOutput(this, 'LambdaFunctionArn', {
            value: rustLambda.functionArn,
            description: 'Production Lambda Function ARN',
            exportName: 'GoddardProdLambdaFunctionArn',
        });
    }
}
exports.RustLambdaStack = RustLambdaStack;
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoicnVzdC1sYW1iZGEtc3RhY2suanMiLCJzb3VyY2VSb290IjoiIiwic291cmNlcyI6WyIuLi8uLi9saWIvcnVzdC1sYW1iZGEtc3RhY2sudHMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6Ijs7O0FBQUEsbUNBQW1DO0FBQ25DLGlEQUFpRDtBQUNqRCx5REFBeUQ7QUFDekQsNkNBQTZDO0FBQzdDLDZCQUE2QjtBQUc3QixNQUFhLGVBQWdCLFNBQVEsR0FBRyxDQUFDLEtBQUs7SUFDNUMsWUFBWSxLQUFnQixFQUFFLEVBQVUsRUFBRSxLQUFzQjtRQUM5RCxLQUFLLENBQUMsS0FBSyxFQUFFLEVBQUUsRUFBRSxLQUFLLENBQUMsQ0FBQztRQUV4QixnQ0FBZ0M7UUFDaEMsNkZBQTZGO1FBQzdGLGtIQUFrSDtRQUNsSCxNQUFNLFVBQVUsR0FBRyxJQUFJLE1BQU0sQ0FBQyxRQUFRLENBQUMsSUFBSSxFQUFFLG1CQUFtQixFQUFFO1lBQ2hFLFlBQVksRUFBRSxjQUFjO1lBQzVCLE9BQU8sRUFBRSxNQUFNLENBQUMsT0FBTyxDQUFDLGVBQWUsRUFBRSxtQ0FBbUM7WUFDNUUsWUFBWSxFQUFFLE1BQU0sQ0FBQyxZQUFZLENBQUMsTUFBTSxFQUFFLGtDQUFrQztZQUM1RSxPQUFPLEVBQUUsV0FBVztZQUNwQixJQUFJLEVBQUUsTUFBTSxDQUFDLElBQUksQ0FBQyxTQUFTLENBQUMsSUFBSSxDQUFDLElBQUksQ0FBQyxTQUFTLEVBQUUsb0RBQW9ELENBQUMsRUFBRTtnQkFDdEcsT0FBTyxFQUFFLENBQUMsSUFBSSxFQUFFLFlBQVksQ0FBQzthQUM5QixDQUFDO1lBQ0YsVUFBVSxFQUFFLEdBQUc7WUFDZixPQUFPLEVBQUUsR0FBRyxDQUFDLFFBQVEsQ0FBQyxPQUFPLENBQUMsRUFBRSxDQUFDO1lBQ2pDLFdBQVcsRUFBRTtnQkFDWCxRQUFRLEVBQUUsTUFBTTthQUNqQjtZQUNELFFBQVEsRUFBRSxJQUFJLElBQUksQ0FBQyxRQUFRLENBQUMsSUFBSSxFQUFFLDJCQUEyQixFQUFFO2dCQUM3RCxZQUFZLEVBQUUsMEJBQTBCO2dCQUN4QyxTQUFTLEVBQUUsSUFBSSxDQUFDLGFBQWEsQ0FBQyxRQUFRO2dCQUN0QyxhQUFhLEVBQUUsR0FBRyxDQUFDLGFBQWEsQ0FBQyxPQUFPO2FBQ3pDLENBQUM7WUFDRixXQUFXLEVBQUUsaUVBQWlFO1NBQy9FLENBQUMsQ0FBQztRQUVILGNBQWM7UUFDZCxNQUFNLEdBQUcsR0FBRyxJQUFJLFVBQVUsQ0FBQyxPQUFPLENBQUMsSUFBSSxFQUFFLGdCQUFnQixFQUFFO1lBQ3pELFdBQVcsRUFBRSx3QkFBd0I7WUFDckMsV0FBVyxFQUFFLDREQUE0RDtZQUN6RSxhQUFhLEVBQUU7Z0JBQ2IsU0FBUyxFQUFFLE1BQU07Z0JBQ2pCLGNBQWMsRUFBRSxJQUFJO2dCQUNwQixjQUFjLEVBQUUsSUFBSTthQUNyQjtZQUNELDJCQUEyQixFQUFFO2dCQUMzQixZQUFZLEVBQUUsVUFBVSxDQUFDLElBQUksQ0FBQyxXQUFXO2dCQUN6QyxZQUFZLEVBQUUsVUFBVSxDQUFDLElBQUksQ0FBQyxXQUFXO2dCQUN6QyxZQUFZLEVBQUUsQ0FBQyxjQUFjLEVBQUUsZUFBZSxFQUFFLGNBQWMsRUFBRSxhQUFhLEVBQUUsV0FBVyxDQUFDO2FBQzVGO1NBQ0YsQ0FBQyxDQUFDO1FBRUgsZ0NBQWdDO1FBQ2hDLE1BQU0saUJBQWlCLEdBQUcsSUFBSSxVQUFVLENBQUMsaUJBQWlCLENBQUMsVUFBVSxFQUFFO1lBQ3JFLEtBQUssRUFBRSxJQUFJO1NBQ1osQ0FBQyxDQUFDO1FBRUgsbUJBQW1CO1FBQ25CLEdBQUcsQ0FBQyxJQUFJLENBQUMsU0FBUyxDQUFDLEtBQUssRUFBRSxpQkFBaUIsQ0FBQyxDQUFDO1FBRTdDLDRDQUE0QztRQUM1QyxNQUFNLGFBQWEsR0FBRyxHQUFHLENBQUMsSUFBSSxDQUFDLFdBQVcsQ0FBQyxVQUFVLENBQUMsQ0FBQztRQUN2RCxhQUFhLENBQUMsU0FBUyxDQUFDLEtBQUssRUFBRSxpQkFBaUIsQ0FBQyxDQUFDO1FBRWxELFVBQVU7UUFDVixJQUFJLEdBQUcsQ0FBQyxTQUFTLENBQUMsSUFBSSxFQUFFLFFBQVEsRUFBRTtZQUNoQyxLQUFLLEVBQUUsR0FBRyxDQUFDLEdBQUc7WUFDZCxXQUFXLEVBQUUsNEJBQTRCO1lBQ3pDLFVBQVUsRUFBRSxtQkFBbUI7U0FDaEMsQ0FBQyxDQUFDO1FBRUgsSUFBSSxHQUFHLENBQUMsU0FBUyxDQUFDLElBQUksRUFBRSxvQkFBb0IsRUFBRTtZQUM1QyxLQUFLLEVBQUUsVUFBVSxDQUFDLFlBQVk7WUFDOUIsV0FBVyxFQUFFLGlDQUFpQztZQUM5QyxVQUFVLEVBQUUsK0JBQStCO1NBQzVDLENBQUMsQ0FBQztRQUVILElBQUksR0FBRyxDQUFDLFNBQVMsQ0FBQyxJQUFJLEVBQUUsbUJBQW1CLEVBQUU7WUFDM0MsS0FBSyxFQUFFLFVBQVUsQ0FBQyxXQUFXO1lBQzdCLFdBQVcsRUFBRSxnQ0FBZ0M7WUFDN0MsVUFBVSxFQUFFLDhCQUE4QjtTQUMzQyxDQUFDLENBQUM7SUFDTCxDQUFDO0NBQ0Y7QUEzRUQsMENBMkVDIiwic291cmNlc0NvbnRlbnQiOlsiaW1wb3J0ICogYXMgY2RrIGZyb20gJ2F3cy1jZGstbGliJztcbmltcG9ydCAqIGFzIGxhbWJkYSBmcm9tICdhd3MtY2RrLWxpYi9hd3MtbGFtYmRhJztcbmltcG9ydCAqIGFzIGFwaWdhdGV3YXkgZnJvbSAnYXdzLWNkay1saWIvYXdzLWFwaWdhdGV3YXknO1xuaW1wb3J0ICogYXMgbG9ncyBmcm9tICdhd3MtY2RrLWxpYi9hd3MtbG9ncyc7XG5pbXBvcnQgKiBhcyBwYXRoIGZyb20gJ3BhdGgnO1xuaW1wb3J0IHsgQ29uc3RydWN0IH0gZnJvbSAnY29uc3RydWN0cyc7XG5cbmV4cG9ydCBjbGFzcyBSdXN0TGFtYmRhU3RhY2sgZXh0ZW5kcyBjZGsuU3RhY2sge1xuICBjb25zdHJ1Y3RvcihzY29wZTogQ29uc3RydWN0LCBpZDogc3RyaW5nLCBwcm9wcz86IGNkay5TdGFja1Byb3BzKSB7XG4gICAgc3VwZXIoc2NvcGUsIGlkLCBwcm9wcyk7XG5cbiAgICAvLyBMYW1iZGEgZnVuY3Rpb24gZm9yIFJ1c3QgY29kZVxuICAgIC8vIFVzaW5nIEFSTTY0IGFyY2hpdGVjdHVyZSBmb3IgdXAgdG8gMzQlIGJldHRlciBwcmljZSBwZXJmb3JtYW5jZSBhbmQgMTklIGJldHRlciBwZXJmb3JtYW5jZVxuICAgIC8vIFNlZTogaHR0cHM6Ly9hd3MuYW1hem9uLmNvbS9ibG9ncy9jb21wdXRlL21pZ3JhdGluZy1hd3MtbGFtYmRhLWZ1bmN0aW9ucy10by1hcm0tYmFzZWQtYXdzLWdyYXZpdG9uMi1wcm9jZXNzb3JzL1xuICAgIGNvbnN0IHJ1c3RMYW1iZGEgPSBuZXcgbGFtYmRhLkZ1bmN0aW9uKHRoaXMsICdHb2RkYXJkUHJvZExhbWJkYScsIHtcbiAgICAgIGZ1bmN0aW9uTmFtZTogJ2dvZGRhcmQtcHJvZCcsXG4gICAgICBydW50aW1lOiBsYW1iZGEuUnVudGltZS5QUk9WSURFRF9BTDIwMjMsIC8vIEFtYXpvbiBMaW51eCAyMDIzIHN1cHBvcnRzIEFSTTY0XG4gICAgICBhcmNoaXRlY3R1cmU6IGxhbWJkYS5BcmNoaXRlY3R1cmUuQVJNXzY0LCAvLyBBV1MgR3Jhdml0b24yIHByb2Nlc3NvciAoQVJNNjQpXG4gICAgICBoYW5kbGVyOiAnYm9vdHN0cmFwJyxcbiAgICAgIGNvZGU6IGxhbWJkYS5Db2RlLmZyb21Bc3NldChwYXRoLmpvaW4oX19kaXJuYW1lLCAnLi4vLi4vbGFtYmRhL2dvZGRhcmQvdGFyZ2V0L2xhbWJkYS9nb2RkYXJkLWJhY2tlbmQnKSwge1xuICAgICAgICBleGNsdWRlOiBbJyoqJywgJyFib290c3RyYXAnXSxcbiAgICAgIH0pLFxuICAgICAgbWVtb3J5U2l6ZTogMjU2LFxuICAgICAgdGltZW91dDogY2RrLkR1cmF0aW9uLnNlY29uZHMoMzApLFxuICAgICAgZW52aXJvbm1lbnQ6IHtcbiAgICAgICAgUlVTVF9MT0c6ICdpbmZvJyxcbiAgICAgIH0sXG4gICAgICBsb2dHcm91cDogbmV3IGxvZ3MuTG9nR3JvdXAodGhpcywgJ0dvZGRhcmRQcm9kTGFtYmRhTG9nR3JvdXAnLCB7XG4gICAgICAgIGxvZ0dyb3VwTmFtZTogJy9hd3MvbGFtYmRhL2dvZGRhcmQtcHJvZCcsXG4gICAgICAgIHJldGVudGlvbjogbG9ncy5SZXRlbnRpb25EYXlzLk9ORV9XRUVLLFxuICAgICAgICByZW1vdmFsUG9saWN5OiBjZGsuUmVtb3ZhbFBvbGljeS5ERVNUUk9ZLFxuICAgICAgfSksXG4gICAgICBkZXNjcmlwdGlvbjogJ0dvZGRhcmQgUHJvZHVjdGlvbiAtIEJhY2tlbmQgTGFtYmRhIGZ1bmN0aW9uIHdpdGggQVBJIGVuZHBvaW50cycsXG4gICAgfSk7XG5cbiAgICAvLyBBUEkgR2F0ZXdheVxuICAgIGNvbnN0IGFwaSA9IG5ldyBhcGlnYXRld2F5LlJlc3RBcGkodGhpcywgJ0dvZGRhcmRQcm9kQXBpJywge1xuICAgICAgcmVzdEFwaU5hbWU6ICdHb2RkYXJkIFByb2R1Y3Rpb24gQVBJJyxcbiAgICAgIGRlc2NyaXB0aW9uOiAnUHJvZHVjdGlvbiBBUEkgR2F0ZXdheSBmb3IgR29kZGFyZCBCYWNrZW5kIExhbWJkYSBmdW5jdGlvbicsXG4gICAgICBkZXBsb3lPcHRpb25zOiB7XG4gICAgICAgIHN0YWdlTmFtZTogJ3Byb2QnLFxuICAgICAgICB0cmFjaW5nRW5hYmxlZDogdHJ1ZSxcbiAgICAgICAgbWV0cmljc0VuYWJsZWQ6IHRydWUsXG4gICAgICB9LFxuICAgICAgZGVmYXVsdENvcnNQcmVmbGlnaHRPcHRpb25zOiB7XG4gICAgICAgIGFsbG93T3JpZ2luczogYXBpZ2F0ZXdheS5Db3JzLkFMTF9PUklHSU5TLFxuICAgICAgICBhbGxvd01ldGhvZHM6IGFwaWdhdGV3YXkuQ29ycy5BTExfTUVUSE9EUyxcbiAgICAgICAgYWxsb3dIZWFkZXJzOiBbJ0NvbnRlbnQtVHlwZScsICdBdXRob3JpemF0aW9uJywgJ3gtcmVxdWVzdC1pZCcsICd4LXNjaG9vbC1pZCcsICd4LWFwaS1rZXknXSxcbiAgICAgIH0sXG4gICAgfSk7XG5cbiAgICAvLyBMYW1iZGEgaW50ZWdyYXRpb24gd2l0aCBwcm94eVxuICAgIGNvbnN0IGxhbWJkYUludGVncmF0aW9uID0gbmV3IGFwaWdhdGV3YXkuTGFtYmRhSW50ZWdyYXRpb24ocnVzdExhbWJkYSwge1xuICAgICAgcHJveHk6IHRydWUsXG4gICAgfSk7XG5cbiAgICAvLyBIYW5kbGUgcm9vdCBwYXRoXG4gICAgYXBpLnJvb3QuYWRkTWV0aG9kKCdBTlknLCBsYW1iZGFJbnRlZ3JhdGlvbik7XG4gICAgXG4gICAgLy8gQ3JlYXRlIHByb3h5IHJlc291cmNlIGZvciBhbGwgb3RoZXIgcGF0aHNcbiAgICBjb25zdCBwcm94eVJlc291cmNlID0gYXBpLnJvb3QuYWRkUmVzb3VyY2UoJ3twcm94eSt9Jyk7XG4gICAgcHJveHlSZXNvdXJjZS5hZGRNZXRob2QoJ0FOWScsIGxhbWJkYUludGVncmF0aW9uKTtcblxuICAgIC8vIE91dHB1dHNcbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnQXBpVXJsJywge1xuICAgICAgdmFsdWU6IGFwaS51cmwsXG4gICAgICBkZXNjcmlwdGlvbjogJ1Byb2R1Y3Rpb24gQVBJIEdhdGV3YXkgVVJMJyxcbiAgICAgIGV4cG9ydE5hbWU6ICdHb2RkYXJkUHJvZEFwaVVybCcsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnTGFtYmRhRnVuY3Rpb25OYW1lJywge1xuICAgICAgdmFsdWU6IHJ1c3RMYW1iZGEuZnVuY3Rpb25OYW1lLFxuICAgICAgZGVzY3JpcHRpb246ICdQcm9kdWN0aW9uIExhbWJkYSBGdW5jdGlvbiBOYW1lJyxcbiAgICAgIGV4cG9ydE5hbWU6ICdHb2RkYXJkUHJvZExhbWJkYUZ1bmN0aW9uTmFtZScsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnTGFtYmRhRnVuY3Rpb25Bcm4nLCB7XG4gICAgICB2YWx1ZTogcnVzdExhbWJkYS5mdW5jdGlvbkFybixcbiAgICAgIGRlc2NyaXB0aW9uOiAnUHJvZHVjdGlvbiBMYW1iZGEgRnVuY3Rpb24gQVJOJyxcbiAgICAgIGV4cG9ydE5hbWU6ICdHb2RkYXJkUHJvZExhbWJkYUZ1bmN0aW9uQXJuJyxcbiAgICB9KTtcbiAgfVxufSJdfQ==