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
            binaryMediaTypes: ['*/*'],
            deployOptions: {
                stageName: stage,
                tracingEnabled: stage === 'prod',
                metricsEnabled: true,
            },
            defaultCorsPreflightOptions: {
                allowOrigins: apigateway.Cors.ALL_ORIGINS,
                allowMethods: apigateway.Cors.ALL_METHODS,
                allowHeaders: ['Content-Type', 'Authorization', 'x-request-id', 'x-school-id', 'x-api-key'],
                exposeHeaders: ['Content-Disposition'],
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
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoicnVzdC1sYW1iZGEtc3RhY2suanMiLCJzb3VyY2VSb290IjoiIiwic291cmNlcyI6WyIuLi8uLi9saWIvcnVzdC1sYW1iZGEtc3RhY2sudHMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6Ijs7O0FBQUEsbUNBQW1DO0FBQ25DLGlEQUFpRDtBQUNqRCx5REFBeUQ7QUFDekQsNkNBQTZDO0FBQzdDLDZCQUE2QjtBQU83QixNQUFhLGVBQWdCLFNBQVEsR0FBRyxDQUFDLEtBQUs7SUFDNUMsWUFBWSxLQUFnQixFQUFFLEVBQVUsRUFBRSxLQUF1QjtRQUMvRCxLQUFLLENBQUMsS0FBSyxFQUFFLEVBQUUsRUFBRSxLQUFLLENBQUMsQ0FBQztRQUV4QixNQUFNLEVBQUUsS0FBSyxFQUFFLEdBQUcsS0FBSyxDQUFDO1FBQ3hCLE1BQU0sU0FBUyxHQUFHLEtBQUssQ0FBQyxXQUFXLEVBQUUsQ0FBQztRQUV0QyxnQ0FBZ0M7UUFDaEMsNkZBQTZGO1FBQzdGLGtIQUFrSDtRQUNsSCxNQUFNLFVBQVUsR0FBRyxJQUFJLE1BQU0sQ0FBQyxRQUFRLENBQUMsSUFBSSxFQUFFLFVBQVUsU0FBUyxRQUFRLEVBQUU7WUFDeEUsWUFBWSxFQUFFLFdBQVcsS0FBSyxFQUFFO1lBQ2hDLE9BQU8sRUFBRSxNQUFNLENBQUMsT0FBTyxDQUFDLGVBQWUsRUFBRSxtQ0FBbUM7WUFDNUUsWUFBWSxFQUFFLE1BQU0sQ0FBQyxZQUFZLENBQUMsTUFBTSxFQUFFLGtDQUFrQztZQUM1RSxPQUFPLEVBQUUsV0FBVztZQUNwQixJQUFJLEVBQUUsTUFBTSxDQUFDLElBQUksQ0FBQyxTQUFTLENBQUMsSUFBSSxDQUFDLElBQUksQ0FBQyxTQUFTLEVBQUUsb0RBQW9ELENBQUMsRUFBRTtnQkFDdEcsT0FBTyxFQUFFLENBQUMsSUFBSSxFQUFFLFlBQVksQ0FBQzthQUM5QixDQUFDO1lBQ0YsVUFBVSxFQUFFLEtBQUssS0FBSyxLQUFLLENBQUMsQ0FBQyxDQUFDLEdBQUcsQ0FBQyxDQUFDLENBQUMsR0FBRztZQUN2QyxPQUFPLEVBQUUsR0FBRyxDQUFDLFFBQVEsQ0FBQyxPQUFPLENBQUMsRUFBRSxDQUFDO1lBQ2pDLFdBQVcsRUFBRTtnQkFDWCxRQUFRLEVBQUUsTUFBTTthQUNqQjtZQUNELFFBQVEsRUFBRSxJQUFJLElBQUksQ0FBQyxRQUFRLENBQUMsSUFBSSxFQUFFLFVBQVUsU0FBUyxnQkFBZ0IsRUFBRTtnQkFDckUsWUFBWSxFQUFFLHVCQUF1QixLQUFLLEVBQUU7Z0JBQzVDLFNBQVMsRUFBRSxJQUFJLENBQUMsYUFBYSxDQUFDLFFBQVE7Z0JBQ3RDLGFBQWEsRUFBRSxHQUFHLENBQUMsYUFBYSxDQUFDLE9BQU87YUFDekMsQ0FBQztZQUNGLFdBQVcsRUFBRSxXQUFXLFNBQVMsK0NBQStDO1NBQ2pGLENBQUMsQ0FBQztRQUVILGNBQWM7UUFDZCxNQUFNLEdBQUcsR0FBRyxJQUFJLFVBQVUsQ0FBQyxPQUFPLENBQUMsSUFBSSxFQUFFLFVBQVUsU0FBUyxLQUFLLEVBQUU7WUFDakUsV0FBVyxFQUFFLFdBQVcsU0FBUyxNQUFNO1lBQ3ZDLFdBQVcsRUFBRSxHQUFHLFNBQVMsa0RBQWtEO1lBQzNFLGdCQUFnQixFQUFFLENBQUMsS0FBSyxDQUFDO1lBQ3pCLGFBQWEsRUFBRTtnQkFDYixTQUFTLEVBQUUsS0FBSztnQkFDaEIsY0FBYyxFQUFFLEtBQUssS0FBSyxNQUFNO2dCQUNoQyxjQUFjLEVBQUUsSUFBSTthQUNyQjtZQUNELDJCQUEyQixFQUFFO2dCQUMzQixZQUFZLEVBQUUsVUFBVSxDQUFDLElBQUksQ0FBQyxXQUFXO2dCQUN6QyxZQUFZLEVBQUUsVUFBVSxDQUFDLElBQUksQ0FBQyxXQUFXO2dCQUN6QyxZQUFZLEVBQUUsQ0FBQyxjQUFjLEVBQUUsZUFBZSxFQUFFLGNBQWMsRUFBRSxhQUFhLEVBQUUsV0FBVyxDQUFDO2dCQUMzRixhQUFhLEVBQUUsQ0FBQyxxQkFBcUIsQ0FBQzthQUN2QztTQUNGLENBQUMsQ0FBQztRQUVILGdDQUFnQztRQUNoQyxNQUFNLGlCQUFpQixHQUFHLElBQUksVUFBVSxDQUFDLGlCQUFpQixDQUFDLFVBQVUsRUFBRTtZQUNyRSxLQUFLLEVBQUUsSUFBSTtTQUNaLENBQUMsQ0FBQztRQUVILG1CQUFtQjtRQUNuQixHQUFHLENBQUMsSUFBSSxDQUFDLFNBQVMsQ0FBQyxLQUFLLEVBQUUsaUJBQWlCLENBQUMsQ0FBQztRQUU3Qyw0Q0FBNEM7UUFDNUMsTUFBTSxhQUFhLEdBQUcsR0FBRyxDQUFDLElBQUksQ0FBQyxXQUFXLENBQUMsVUFBVSxDQUFDLENBQUM7UUFDdkQsYUFBYSxDQUFDLFNBQVMsQ0FBQyxLQUFLLEVBQUUsaUJBQWlCLENBQUMsQ0FBQztRQUVsRCxVQUFVO1FBQ1YsSUFBSSxHQUFHLENBQUMsU0FBUyxDQUFDLElBQUksRUFBRSxRQUFRLEVBQUU7WUFDaEMsS0FBSyxFQUFFLEdBQUcsQ0FBQyxHQUFHO1lBQ2QsV0FBVyxFQUFFLEdBQUcsU0FBUyxrQkFBa0I7WUFDM0MsVUFBVSxFQUFFLFVBQVUsU0FBUyxRQUFRO1NBQ3hDLENBQUMsQ0FBQztRQUVILElBQUksR0FBRyxDQUFDLFNBQVMsQ0FBQyxJQUFJLEVBQUUsb0JBQW9CLEVBQUU7WUFDNUMsS0FBSyxFQUFFLFVBQVUsQ0FBQyxZQUFZO1lBQzlCLFdBQVcsRUFBRSxHQUFHLFNBQVMsdUJBQXVCO1lBQ2hELFVBQVUsRUFBRSxVQUFVLFNBQVMsb0JBQW9CO1NBQ3BELENBQUMsQ0FBQztRQUVILElBQUksR0FBRyxDQUFDLFNBQVMsQ0FBQyxJQUFJLEVBQUUsbUJBQW1CLEVBQUU7WUFDM0MsS0FBSyxFQUFFLFVBQVUsQ0FBQyxXQUFXO1lBQzdCLFdBQVcsRUFBRSxHQUFHLFNBQVMsc0JBQXNCO1lBQy9DLFVBQVUsRUFBRSxVQUFVLFNBQVMsbUJBQW1CO1NBQ25ELENBQUMsQ0FBQztJQUNMLENBQUM7Q0FDRjtBQWhGRCwwQ0FnRkMiLCJzb3VyY2VzQ29udGVudCI6WyJpbXBvcnQgKiBhcyBjZGsgZnJvbSAnYXdzLWNkay1saWInO1xuaW1wb3J0ICogYXMgbGFtYmRhIGZyb20gJ2F3cy1jZGstbGliL2F3cy1sYW1iZGEnO1xuaW1wb3J0ICogYXMgYXBpZ2F0ZXdheSBmcm9tICdhd3MtY2RrLWxpYi9hd3MtYXBpZ2F0ZXdheSc7XG5pbXBvcnQgKiBhcyBsb2dzIGZyb20gJ2F3cy1jZGstbGliL2F3cy1sb2dzJztcbmltcG9ydCAqIGFzIHBhdGggZnJvbSAncGF0aCc7XG5pbXBvcnQgeyBDb25zdHJ1Y3QgfSBmcm9tICdjb25zdHJ1Y3RzJztcblxuaW50ZXJmYWNlIEdvZGRhclN0YWNrUHJvcHMgZXh0ZW5kcyBjZGsuU3RhY2tQcm9wcyB7XG4gIHN0YWdlOiAnZGV2JyB8ICdwcm9kJztcbn1cblxuZXhwb3J0IGNsYXNzIFJ1c3RMYW1iZGFTdGFjayBleHRlbmRzIGNkay5TdGFjayB7XG4gIGNvbnN0cnVjdG9yKHNjb3BlOiBDb25zdHJ1Y3QsIGlkOiBzdHJpbmcsIHByb3BzOiBHb2RkYXJTdGFja1Byb3BzKSB7XG4gICAgc3VwZXIoc2NvcGUsIGlkLCBwcm9wcyk7XG5cbiAgICBjb25zdCB7IHN0YWdlIH0gPSBwcm9wcztcbiAgICBjb25zdCBzdGFnZU5hbWUgPSBzdGFnZS50b1VwcGVyQ2FzZSgpO1xuXG4gICAgLy8gTGFtYmRhIGZ1bmN0aW9uIGZvciBSdXN0IGNvZGVcbiAgICAvLyBVc2luZyBBUk02NCBhcmNoaXRlY3R1cmUgZm9yIHVwIHRvIDM0JSBiZXR0ZXIgcHJpY2UgcGVyZm9ybWFuY2UgYW5kIDE5JSBiZXR0ZXIgcGVyZm9ybWFuY2VcbiAgICAvLyBTZWU6IGh0dHBzOi8vYXdzLmFtYXpvbi5jb20vYmxvZ3MvY29tcHV0ZS9taWdyYXRpbmctYXdzLWxhbWJkYS1mdW5jdGlvbnMtdG8tYXJtLWJhc2VkLWF3cy1ncmF2aXRvbjItcHJvY2Vzc29ycy9cbiAgICBjb25zdCBydXN0TGFtYmRhID0gbmV3IGxhbWJkYS5GdW5jdGlvbih0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfUxhbWJkYWAsIHtcbiAgICAgIGZ1bmN0aW9uTmFtZTogYGdvZGRhcmQtJHtzdGFnZX1gLFxuICAgICAgcnVudGltZTogbGFtYmRhLlJ1bnRpbWUuUFJPVklERURfQUwyMDIzLCAvLyBBbWF6b24gTGludXggMjAyMyBzdXBwb3J0cyBBUk02NFxuICAgICAgYXJjaGl0ZWN0dXJlOiBsYW1iZGEuQXJjaGl0ZWN0dXJlLkFSTV82NCwgLy8gQVdTIEdyYXZpdG9uMiBwcm9jZXNzb3IgKEFSTTY0KVxuICAgICAgaGFuZGxlcjogJ2Jvb3RzdHJhcCcsXG4gICAgICBjb2RlOiBsYW1iZGEuQ29kZS5mcm9tQXNzZXQocGF0aC5qb2luKF9fZGlybmFtZSwgJy4uLy4uL2xhbWJkYS9nb2RkYXJkL3RhcmdldC9sYW1iZGEvZ29kZGFyZC1iYWNrZW5kJyksIHtcbiAgICAgICAgZXhjbHVkZTogWycqKicsICchYm9vdHN0cmFwJ10sXG4gICAgICB9KSxcbiAgICAgIG1lbW9yeVNpemU6IHN0YWdlID09PSAnZGV2JyA/IDEyOCA6IDI1NixcbiAgICAgIHRpbWVvdXQ6IGNkay5EdXJhdGlvbi5zZWNvbmRzKDMwKSxcbiAgICAgIGVudmlyb25tZW50OiB7XG4gICAgICAgIFJVU1RfTE9HOiAnaW5mbycsXG4gICAgICB9LFxuICAgICAgbG9nR3JvdXA6IG5ldyBsb2dzLkxvZ0dyb3VwKHRoaXMsIGBHb2RkYXJkJHtzdGFnZU5hbWV9TGFtYmRhTG9nR3JvdXBgLCB7XG4gICAgICAgIGxvZ0dyb3VwTmFtZTogYC9hd3MvbGFtYmRhL2dvZGRhcmQtJHtzdGFnZX1gLFxuICAgICAgICByZXRlbnRpb246IGxvZ3MuUmV0ZW50aW9uRGF5cy5PTkVfV0VFSyxcbiAgICAgICAgcmVtb3ZhbFBvbGljeTogY2RrLlJlbW92YWxQb2xpY3kuREVTVFJPWSxcbiAgICAgIH0pLFxuICAgICAgZGVzY3JpcHRpb246IGBHb2RkYXJkICR7c3RhZ2VOYW1lfSAtIEJhY2tlbmQgTGFtYmRhIGZ1bmN0aW9uIHdpdGggQVBJIGVuZHBvaW50c2AsXG4gICAgfSk7XG5cbiAgICAvLyBBUEkgR2F0ZXdheVxuICAgIGNvbnN0IGFwaSA9IG5ldyBhcGlnYXRld2F5LlJlc3RBcGkodGhpcywgYEdvZGRhcmQke3N0YWdlTmFtZX1BcGlgLCB7XG4gICAgICByZXN0QXBpTmFtZTogYEdvZGRhcmQgJHtzdGFnZU5hbWV9IEFQSWAsXG4gICAgICBkZXNjcmlwdGlvbjogYCR7c3RhZ2VOYW1lfSBBUEkgR2F0ZXdheSBmb3IgR29kZGFyZCBCYWNrZW5kIExhbWJkYSBmdW5jdGlvbmAsXG4gICAgICBiaW5hcnlNZWRpYVR5cGVzOiBbJyovKiddLFxuICAgICAgZGVwbG95T3B0aW9uczoge1xuICAgICAgICBzdGFnZU5hbWU6IHN0YWdlLFxuICAgICAgICB0cmFjaW5nRW5hYmxlZDogc3RhZ2UgPT09ICdwcm9kJyxcbiAgICAgICAgbWV0cmljc0VuYWJsZWQ6IHRydWUsXG4gICAgICB9LFxuICAgICAgZGVmYXVsdENvcnNQcmVmbGlnaHRPcHRpb25zOiB7XG4gICAgICAgIGFsbG93T3JpZ2luczogYXBpZ2F0ZXdheS5Db3JzLkFMTF9PUklHSU5TLFxuICAgICAgICBhbGxvd01ldGhvZHM6IGFwaWdhdGV3YXkuQ29ycy5BTExfTUVUSE9EUyxcbiAgICAgICAgYWxsb3dIZWFkZXJzOiBbJ0NvbnRlbnQtVHlwZScsICdBdXRob3JpemF0aW9uJywgJ3gtcmVxdWVzdC1pZCcsICd4LXNjaG9vbC1pZCcsICd4LWFwaS1rZXknXSxcbiAgICAgICAgZXhwb3NlSGVhZGVyczogWydDb250ZW50LURpc3Bvc2l0aW9uJ10sXG4gICAgICB9LFxuICAgIH0pO1xuXG4gICAgLy8gTGFtYmRhIGludGVncmF0aW9uIHdpdGggcHJveHlcbiAgICBjb25zdCBsYW1iZGFJbnRlZ3JhdGlvbiA9IG5ldyBhcGlnYXRld2F5LkxhbWJkYUludGVncmF0aW9uKHJ1c3RMYW1iZGEsIHtcbiAgICAgIHByb3h5OiB0cnVlLFxuICAgIH0pO1xuXG4gICAgLy8gSGFuZGxlIHJvb3QgcGF0aFxuICAgIGFwaS5yb290LmFkZE1ldGhvZCgnQU5ZJywgbGFtYmRhSW50ZWdyYXRpb24pO1xuXG4gICAgLy8gQ3JlYXRlIHByb3h5IHJlc291cmNlIGZvciBhbGwgb3RoZXIgcGF0aHNcbiAgICBjb25zdCBwcm94eVJlc291cmNlID0gYXBpLnJvb3QuYWRkUmVzb3VyY2UoJ3twcm94eSt9Jyk7XG4gICAgcHJveHlSZXNvdXJjZS5hZGRNZXRob2QoJ0FOWScsIGxhbWJkYUludGVncmF0aW9uKTtcblxuICAgIC8vIE91dHB1dHNcbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnQXBpVXJsJywge1xuICAgICAgdmFsdWU6IGFwaS51cmwsXG4gICAgICBkZXNjcmlwdGlvbjogYCR7c3RhZ2VOYW1lfSBBUEkgR2F0ZXdheSBVUkxgLFxuICAgICAgZXhwb3J0TmFtZTogYEdvZGRhcmQke3N0YWdlTmFtZX1BcGlVcmxgLFxuICAgIH0pO1xuXG4gICAgbmV3IGNkay5DZm5PdXRwdXQodGhpcywgJ0xhbWJkYUZ1bmN0aW9uTmFtZScsIHtcbiAgICAgIHZhbHVlOiBydXN0TGFtYmRhLmZ1bmN0aW9uTmFtZSxcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IExhbWJkYSBGdW5jdGlvbiBOYW1lYCxcbiAgICAgIGV4cG9ydE5hbWU6IGBHb2RkYXJkJHtzdGFnZU5hbWV9TGFtYmRhRnVuY3Rpb25OYW1lYCxcbiAgICB9KTtcblxuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdMYW1iZGFGdW5jdGlvbkFybicsIHtcbiAgICAgIHZhbHVlOiBydXN0TGFtYmRhLmZ1bmN0aW9uQXJuLFxuICAgICAgZGVzY3JpcHRpb246IGAke3N0YWdlTmFtZX0gTGFtYmRhIEZ1bmN0aW9uIEFSTmAsXG4gICAgICBleHBvcnROYW1lOiBgR29kZGFyZCR7c3RhZ2VOYW1lfUxhbWJkYUZ1bmN0aW9uQXJuYCxcbiAgICB9KTtcbiAgfVxufVxuIl19