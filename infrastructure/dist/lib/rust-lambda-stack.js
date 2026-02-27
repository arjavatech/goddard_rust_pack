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
        const { stage } = props;
        const stageName = stage.toUpperCase();
        // Lambda function for Rust code
        // Using ARM64 architecture for up to 34% better price performance and 19% better performance
        // See: https://aws.amazon.com/blogs/compute/migrating-aws-lambda-functions-to-arm-based-aws-graviton2-processors/
        const rustLambda = new lambda.Function(this, `Goddard${stageName}Lambda`, {
            functionName: `goddard-${stage}`,
            runtime: lambda.Runtime.PROVIDED_AL2023, // Amazon Linux 2023 supports ARM64
            architecture: lambda.Architecture.ARM_64, // AWS Graviton2 processor (ARM64)
            handler: 'bootstrap',
            code: lambda.Code.fromAsset(path.join(__dirname, '../../lambda/goddard/target/lambda/goddard-backend'), {
                exclude: ['**', '!bootstrap'],
            }),
            memorySize: stage === 'dev' ? 128 : 256,
            timeout: cdk.Duration.seconds(30),
            environment: {
                RUST_LOG: 'info',
            },
            logGroup: new logs.LogGroup(this, `Goddard${stageName}LambdaLogGroup`, {
                logGroupName: `/aws/lambda/goddard-${stage}`,
                retention: logs.RetentionDays.ONE_WEEK,
                removalPolicy: cdk.RemovalPolicy.DESTROY,
            }),
            description: `Goddard ${stageName} - Backend Lambda function with API endpoints`,
        });
        // API Gateway
        const api = new apigateway.RestApi(this, `Goddard${stageName}Api`, {
            restApiName: `Goddard ${stageName} API`,
            description: `${stageName} API Gateway for Goddard Backend Lambda function`,
            deployOptions: {
                stageName: stage,
                tracingEnabled: stage === 'prod',
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
            description: `${stageName} API Gateway URL`,
            exportName: `Goddard${stageName}ApiUrl`,
        });
        new cdk.CfnOutput(this, 'LambdaFunctionName', {
            value: rustLambda.functionName,
            description: `${stageName} Lambda Function Name`,
            exportName: `Goddard${stageName}LambdaFunctionName`,
        });
        new cdk.CfnOutput(this, 'LambdaFunctionArn', {
            value: rustLambda.functionArn,
            description: `${stageName} Lambda Function ARN`,
            exportName: `Goddard${stageName}LambdaFunctionArn`,
        });
    }
}
exports.RustLambdaStack = RustLambdaStack;
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoicnVzdC1sYW1iZGEtc3RhY2suanMiLCJzb3VyY2VSb290IjoiIiwic291cmNlcyI6WyIuLi8uLi9saWIvcnVzdC1sYW1iZGEtc3RhY2sudHMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6Ijs7O0FBQUEsbUNBQW1DO0FBQ25DLGlEQUFpRDtBQUNqRCx5REFBeUQ7QUFDekQsNkNBQTZDO0FBQzdDLDZCQUE2QjtBQU83QixNQUFhLGVBQWdCLFNBQVEsR0FBRyxDQUFDLEtBQUs7SUFDNUMsWUFBWSxLQUFnQixFQUFFLEVBQVUsRUFBRSxLQUF1QjtRQUMvRCxLQUFLLENBQUMsS0FBSyxFQUFFLEVBQUUsRUFBRSxLQUFLLENBQUMsQ0FBQztRQUV4QixNQUFNLEVBQUUsS0FBSyxFQUFFLEdBQUcsS0FBSyxDQUFDO1FBQ3hCLE1BQU0sU0FBUyxHQUFHLEtBQUssQ0FBQyxXQUFXLEVBQUUsQ0FBQztRQUV0QyxnQ0FBZ0M7UUFDaEMsNkZBQTZGO1FBQzdGLGtIQUFrSDtRQUNsSCxNQUFNLFVBQVUsR0FBRyxJQUFJLE1BQU0sQ0FBQyxRQUFRLENBQUMsSUFBSSxFQUFFLFVBQVUsU0FBUyxRQUFRLEVBQUU7WUFDeEUsWUFBWSxFQUFFLFdBQVcsS0FBSyxFQUFFO1lBQ2hDLE9BQU8sRUFBRSxNQUFNLENBQUMsT0FBTyxDQUFDLGVBQWUsRUFBRSxtQ0FBbUM7WUFDNUUsWUFBWSxFQUFFLE1BQU0sQ0FBQyxZQUFZLENBQUMsTUFBTSxFQUFFLGtDQUFrQztZQUM1RSxPQUFPLEVBQUUsV0FBVztZQUNwQixJQUFJLEVBQUUsTUFBTSxDQUFDLElBQUksQ0FBQyxTQUFTLENBQUMsSUFBSSxDQUFDLElBQUksQ0FBQyxTQUFTLEVBQUUsb0RBQW9ELENBQUMsRUFBRTtnQkFDdEcsT0FBTyxFQUFFLENBQUMsSUFBSSxFQUFFLFlBQVksQ0FBQzthQUM5QixDQUFDO1lBQ0YsVUFBVSxFQUFFLEtBQUssS0FBSyxLQUFLLENBQUMsQ0FBQyxDQUFDLEdBQUcsQ0FBQyxDQUFDLENBQUMsR0FBRztZQUN2QyxPQUFPLEVBQUUsR0FBRyxDQUFDLFFBQVEsQ0FBQyxPQUFPLENBQUMsRUFBRSxDQUFDO1lBQ2pDLFdBQVcsRUFBRTtnQkFDWCxRQUFRLEVBQUUsTUFBTTthQUNqQjtZQUNELFFBQVEsRUFBRSxJQUFJLElBQUksQ0FBQyxRQUFRLENBQUMsSUFBSSxFQUFFLFVBQVUsU0FBUyxnQkFBZ0IsRUFBRTtnQkFDckUsWUFBWSxFQUFFLHVCQUF1QixLQUFLLEVBQUU7Z0JBQzVDLFNBQVMsRUFBRSxJQUFJLENBQUMsYUFBYSxDQUFDLFFBQVE7Z0JBQ3RDLGFBQWEsRUFBRSxHQUFHLENBQUMsYUFBYSxDQUFDLE9BQU87YUFDekMsQ0FBQztZQUNGLFdBQVcsRUFBRSxXQUFXLFNBQVMsK0NBQStDO1NBQ2pGLENBQUMsQ0FBQztRQUVILGNBQWM7UUFDZCxNQUFNLEdBQUcsR0FBRyxJQUFJLFVBQVUsQ0FBQyxPQUFPLENBQUMsSUFBSSxFQUFFLFVBQVUsU0FBUyxLQUFLLEVBQUU7WUFDakUsV0FBVyxFQUFFLFdBQVcsU0FBUyxNQUFNO1lBQ3ZDLFdBQVcsRUFBRSxHQUFHLFNBQVMsa0RBQWtEO1lBQzNFLGFBQWEsRUFBRTtnQkFDYixTQUFTLEVBQUUsS0FBSztnQkFDaEIsY0FBYyxFQUFFLEtBQUssS0FBSyxNQUFNO2dCQUNoQyxjQUFjLEVBQUUsSUFBSTthQUNyQjtZQUNELDJCQUEyQixFQUFFO2dCQUMzQixZQUFZLEVBQUUsVUFBVSxDQUFDLElBQUksQ0FBQyxXQUFXO2dCQUN6QyxZQUFZLEVBQUUsVUFBVSxDQUFDLElBQUksQ0FBQyxXQUFXO2dCQUN6QyxZQUFZLEVBQUUsQ0FBQyxjQUFjLEVBQUUsZUFBZSxFQUFFLGNBQWMsRUFBRSxhQUFhLEVBQUUsV0FBVyxDQUFDO2FBQzVGO1NBQ0YsQ0FBQyxDQUFDO1FBRUgsZ0NBQWdDO1FBQ2hDLE1BQU0saUJBQWlCLEdBQUcsSUFBSSxVQUFVLENBQUMsaUJBQWlCLENBQUMsVUFBVSxFQUFFO1lBQ3JFLEtBQUssRUFBRSxJQUFJO1NBQ1osQ0FBQyxDQUFDO1FBRUgsbUJBQW1CO1FBQ25CLEdBQUcsQ0FBQyxJQUFJLENBQUMsU0FBUyxDQUFDLEtBQUssRUFBRSxpQkFBaUIsQ0FBQyxDQUFDO1FBRTdDLDRDQUE0QztRQUM1QyxNQUFNLGFBQWEsR0FBRyxHQUFHLENBQUMsSUFBSSxDQUFDLFdBQVcsQ0FBQyxVQUFVLENBQUMsQ0FBQztRQUN2RCxhQUFhLENBQUMsU0FBUyxDQUFDLEtBQUssRUFBRSxpQkFBaUIsQ0FBQyxDQUFDO1FBRWxELFVBQVU7UUFDVixJQUFJLEdBQUcsQ0FBQyxTQUFTLENBQUMsSUFBSSxFQUFFLFFBQVEsRUFBRTtZQUNoQyxLQUFLLEVBQUUsR0FBRyxDQUFDLEdBQUc7WUFDZCxXQUFXLEVBQUUsR0FBRyxTQUFTLGtCQUFrQjtZQUMzQyxVQUFVLEVBQUUsVUFBVSxTQUFTLFFBQVE7U0FDeEMsQ0FBQyxDQUFDO1FBRUgsSUFBSSxHQUFHLENBQUMsU0FBUyxDQUFDLElBQUksRUFBRSxvQkFBb0IsRUFBRTtZQUM1QyxLQUFLLEVBQUUsVUFBVSxDQUFDLFlBQVk7WUFDOUIsV0FBVyxFQUFFLEdBQUcsU0FBUyx1QkFBdUI7WUFDaEQsVUFBVSxFQUFFLFVBQVUsU0FBUyxvQkFBb0I7U0FDcEQsQ0FBQyxDQUFDO1FBRUgsSUFBSSxHQUFHLENBQUMsU0FBUyxDQUFDLElBQUksRUFBRSxtQkFBbUIsRUFBRTtZQUMzQyxLQUFLLEVBQUUsVUFBVSxDQUFDLFdBQVc7WUFDN0IsV0FBVyxFQUFFLEdBQUcsU0FBUyxzQkFBc0I7WUFDL0MsVUFBVSxFQUFFLFVBQVUsU0FBUyxtQkFBbUI7U0FDbkQsQ0FBQyxDQUFDO0lBQ0wsQ0FBQztDQUNGO0FBOUVELDBDQThFQyIsInNvdXJjZXNDb250ZW50IjpbImltcG9ydCAqIGFzIGNkayBmcm9tICdhd3MtY2RrLWxpYic7XG5pbXBvcnQgKiBhcyBsYW1iZGEgZnJvbSAnYXdzLWNkay1saWIvYXdzLWxhbWJkYSc7XG5pbXBvcnQgKiBhcyBhcGlnYXRld2F5IGZyb20gJ2F3cy1jZGstbGliL2F3cy1hcGlnYXRld2F5JztcbmltcG9ydCAqIGFzIGxvZ3MgZnJvbSAnYXdzLWNkay1saWIvYXdzLWxvZ3MnO1xuaW1wb3J0ICogYXMgcGF0aCBmcm9tICdwYXRoJztcbmltcG9ydCB7IENvbnN0cnVjdCB9IGZyb20gJ2NvbnN0cnVjdHMnO1xuXG5pbnRlcmZhY2UgR29kZGFyU3RhY2tQcm9wcyBleHRlbmRzIGNkay5TdGFja1Byb3BzIHtcbiAgc3RhZ2U6ICdkZXYnIHwgJ3Byb2QnO1xufVxuXG5leHBvcnQgY2xhc3MgUnVzdExhbWJkYVN0YWNrIGV4dGVuZHMgY2RrLlN0YWNrIHtcbiAgY29uc3RydWN0b3Ioc2NvcGU6IENvbnN0cnVjdCwgaWQ6IHN0cmluZywgcHJvcHM6IEdvZGRhclN0YWNrUHJvcHMpIHtcbiAgICBzdXBlcihzY29wZSwgaWQsIHByb3BzKTtcblxuICAgIGNvbnN0IHsgc3RhZ2UgfSA9IHByb3BzO1xuICAgIGNvbnN0IHN0YWdlTmFtZSA9IHN0YWdlLnRvVXBwZXJDYXNlKCk7XG5cbiAgICAvLyBMYW1iZGEgZnVuY3Rpb24gZm9yIFJ1c3QgY29kZVxuICAgIC8vIFVzaW5nIEFSTTY0IGFyY2hpdGVjdHVyZSBmb3IgdXAgdG8gMzQlIGJldHRlciBwcmljZSBwZXJmb3JtYW5jZSBhbmQgMTklIGJldHRlciBwZXJmb3JtYW5jZVxuICAgIC8vIFNlZTogaHR0cHM6Ly9hd3MuYW1hem9uLmNvbS9ibG9ncy9jb21wdXRlL21pZ3JhdGluZy1hd3MtbGFtYmRhLWZ1bmN0aW9ucy10by1hcm0tYmFzZWQtYXdzLWdyYXZpdG9uMi1wcm9jZXNzb3JzL1xuICAgIGNvbnN0IHJ1c3RMYW1iZGEgPSBuZXcgbGFtYmRhLkZ1bmN0aW9uKHRoaXMsIGBHb2RkYXJkJHtzdGFnZU5hbWV9TGFtYmRhYCwge1xuICAgICAgZnVuY3Rpb25OYW1lOiBgZ29kZGFyZC0ke3N0YWdlfWAsXG4gICAgICBydW50aW1lOiBsYW1iZGEuUnVudGltZS5QUk9WSURFRF9BTDIwMjMsIC8vIEFtYXpvbiBMaW51eCAyMDIzIHN1cHBvcnRzIEFSTTY0XG4gICAgICBhcmNoaXRlY3R1cmU6IGxhbWJkYS5BcmNoaXRlY3R1cmUuQVJNXzY0LCAvLyBBV1MgR3Jhdml0b24yIHByb2Nlc3NvciAoQVJNNjQpXG4gICAgICBoYW5kbGVyOiAnYm9vdHN0cmFwJyxcbiAgICAgIGNvZGU6IGxhbWJkYS5Db2RlLmZyb21Bc3NldChwYXRoLmpvaW4oX19kaXJuYW1lLCAnLi4vLi4vbGFtYmRhL2dvZGRhcmQvdGFyZ2V0L2xhbWJkYS9nb2RkYXJkLWJhY2tlbmQnKSwge1xuICAgICAgICBleGNsdWRlOiBbJyoqJywgJyFib290c3RyYXAnXSxcbiAgICAgIH0pLFxuICAgICAgbWVtb3J5U2l6ZTogc3RhZ2UgPT09ICdkZXYnID8gMTI4IDogMjU2LFxuICAgICAgdGltZW91dDogY2RrLkR1cmF0aW9uLnNlY29uZHMoMzApLFxuICAgICAgZW52aXJvbm1lbnQ6IHtcbiAgICAgICAgUlVTVF9MT0c6ICdpbmZvJyxcbiAgICAgIH0sXG4gICAgICBsb2dHcm91cDogbmV3IGxvZ3MuTG9nR3JvdXAodGhpcywgYEdvZGRhcmQke3N0YWdlTmFtZX1MYW1iZGFMb2dHcm91cGAsIHtcbiAgICAgICAgbG9nR3JvdXBOYW1lOiBgL2F3cy9sYW1iZGEvZ29kZGFyZC0ke3N0YWdlfWAsXG4gICAgICAgIHJldGVudGlvbjogbG9ncy5SZXRlbnRpb25EYXlzLk9ORV9XRUVLLFxuICAgICAgICByZW1vdmFsUG9saWN5OiBjZGsuUmVtb3ZhbFBvbGljeS5ERVNUUk9ZLFxuICAgICAgfSksXG4gICAgICBkZXNjcmlwdGlvbjogYEdvZGRhcmQgJHtzdGFnZU5hbWV9IC0gQmFja2VuZCBMYW1iZGEgZnVuY3Rpb24gd2l0aCBBUEkgZW5kcG9pbnRzYCxcbiAgICB9KTtcblxuICAgIC8vIEFQSSBHYXRld2F5XG4gICAgY29uc3QgYXBpID0gbmV3IGFwaWdhdGV3YXkuUmVzdEFwaSh0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfUFwaWAsIHtcbiAgICAgIHJlc3RBcGlOYW1lOiBgR29kZGFyZCAke3N0YWdlTmFtZX0gQVBJYCxcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IEFQSSBHYXRld2F5IGZvciBHb2RkYXJkIEJhY2tlbmQgTGFtYmRhIGZ1bmN0aW9uYCxcbiAgICAgIGRlcGxveU9wdGlvbnM6IHtcbiAgICAgICAgc3RhZ2VOYW1lOiBzdGFnZSxcbiAgICAgICAgdHJhY2luZ0VuYWJsZWQ6IHN0YWdlID09PSAncHJvZCcsXG4gICAgICAgIG1ldHJpY3NFbmFibGVkOiB0cnVlLFxuICAgICAgfSxcbiAgICAgIGRlZmF1bHRDb3JzUHJlZmxpZ2h0T3B0aW9uczoge1xuICAgICAgICBhbGxvd09yaWdpbnM6IGFwaWdhdGV3YXkuQ29ycy5BTExfT1JJR0lOUyxcbiAgICAgICAgYWxsb3dNZXRob2RzOiBhcGlnYXRld2F5LkNvcnMuQUxMX01FVEhPRFMsXG4gICAgICAgIGFsbG93SGVhZGVyczogWydDb250ZW50LVR5cGUnLCAnQXV0aG9yaXphdGlvbicsICd4LXJlcXVlc3QtaWQnLCAneC1zY2hvb2wtaWQnLCAneC1hcGkta2V5J10sXG4gICAgICB9LFxuICAgIH0pO1xuXG4gICAgLy8gTGFtYmRhIGludGVncmF0aW9uIHdpdGggcHJveHlcbiAgICBjb25zdCBsYW1iZGFJbnRlZ3JhdGlvbiA9IG5ldyBhcGlnYXRld2F5LkxhbWJkYUludGVncmF0aW9uKHJ1c3RMYW1iZGEsIHtcbiAgICAgIHByb3h5OiB0cnVlLFxuICAgIH0pO1xuXG4gICAgLy8gSGFuZGxlIHJvb3QgcGF0aFxuICAgIGFwaS5yb290LmFkZE1ldGhvZCgnQU5ZJywgbGFtYmRhSW50ZWdyYXRpb24pO1xuXG4gICAgLy8gQ3JlYXRlIHByb3h5IHJlc291cmNlIGZvciBhbGwgb3RoZXIgcGF0aHNcbiAgICBjb25zdCBwcm94eVJlc291cmNlID0gYXBpLnJvb3QuYWRkUmVzb3VyY2UoJ3twcm94eSt9Jyk7XG4gICAgcHJveHlSZXNvdXJjZS5hZGRNZXRob2QoJ0FOWScsIGxhbWJkYUludGVncmF0aW9uKTtcblxuICAgIC8vIE91dHB1dHNcbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnQXBpVXJsJywge1xuICAgICAgdmFsdWU6IGFwaS51cmwsXG4gICAgICBkZXNjcmlwdGlvbjogYCR7c3RhZ2VOYW1lfSBBUEkgR2F0ZXdheSBVUkxgLFxuICAgICAgZXhwb3J0TmFtZTogYEdvZGRhcmQke3N0YWdlTmFtZX1BcGlVcmxgLFxuICAgIH0pO1xuXG4gICAgbmV3IGNkay5DZm5PdXRwdXQodGhpcywgJ0xhbWJkYUZ1bmN0aW9uTmFtZScsIHtcbiAgICAgIHZhbHVlOiBydXN0TGFtYmRhLmZ1bmN0aW9uTmFtZSxcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IExhbWJkYSBGdW5jdGlvbiBOYW1lYCxcbiAgICAgIGV4cG9ydE5hbWU6IGBHb2RkYXJkJHtzdGFnZU5hbWV9TGFtYmRhRnVuY3Rpb25OYW1lYCxcbiAgICB9KTtcblxuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdMYW1iZGFGdW5jdGlvbkFybicsIHtcbiAgICAgIHZhbHVlOiBydXN0TGFtYmRhLmZ1bmN0aW9uQXJuLFxuICAgICAgZGVzY3JpcHRpb246IGAke3N0YWdlTmFtZX0gTGFtYmRhIEZ1bmN0aW9uIEFSTmAsXG4gICAgICBleHBvcnROYW1lOiBgR29kZGFyZCR7c3RhZ2VOYW1lfUxhbWJkYUZ1bmN0aW9uQXJuYCxcbiAgICB9KTtcbiAgfVxufVxuIl19