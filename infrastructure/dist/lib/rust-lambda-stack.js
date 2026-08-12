"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.RustLambdaStack = void 0;
const cdk = require("aws-cdk-lib");
const lambda = require("aws-cdk-lib/aws-lambda");
const apigateway = require("aws-cdk-lib/aws-apigateway");
const logs = require("aws-cdk-lib/aws-logs");
const s3 = require("aws-cdk-lib/aws-s3");
const path = require("path");
class RustLambdaStack extends cdk.Stack {
    constructor(scope, id, props) {
        super(scope, id, props);
        const { stage } = props;
        const stageName = stage.toUpperCase();
        // S3 bucket for product image uploads
        const uploadsBucket = new s3.Bucket(this, `Goddard${stageName}UploadsBucket`, {
            bucketName: `goddard-uploads-${stage}`,
            publicReadAccess: true,
            blockPublicAccess: s3.BlockPublicAccess.BLOCK_ACLS,
            cors: [
                {
                    allowedMethods: [s3.HttpMethods.GET, s3.HttpMethods.PUT],
                    allowedOrigins: ['*'],
                    allowedHeaders: ['*'],
                },
            ],
            removalPolicy: cdk.RemovalPolicy.RETAIN,
        });
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
                S3_UPLOAD_BUCKET: uploadsBucket.bucketName,
                S3_BASE_URL: `https://${uploadsBucket.bucketRegionalDomainName}`,
            },
            logGroup: new logs.LogGroup(this, `Goddard${stageName}LambdaLogGroup`, {
                logGroupName: `/aws/lambda/goddard-${stage}`,
                retention: logs.RetentionDays.ONE_WEEK,
                removalPolicy: cdk.RemovalPolicy.DESTROY,
            }),
            description: `Goddard ${stageName} - Backend Lambda function with API endpoints`,
        });
        // Grant Lambda write access to the uploads bucket
        uploadsBucket.grantPut(rustLambda);
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
            // CORS is handled entirely by Lambda middleware (cors.rs).
            // Do NOT use defaultCorsPreflightOptions here — it creates a MOCK
            // integration for OPTIONS that conflicts with binaryMediaTypes: ['*/*'],
            // causing API Gateway to corrupt/strip CORS headers from preflight responses.
        });
        // Lambda integration with proxy
        const lambdaIntegration = new apigateway.LambdaIntegration(rustLambda, {
            proxy: true,
        });
        // Handle root path
        api.root.addMethod('ANY', lambdaIntegration);
        // Explicit OPTIONS on root — ANY does NOT forward OPTIONS in REST API
        api.root.addMethod('OPTIONS', lambdaIntegration);
        // Create proxy resource for all other paths
        const proxyResource = api.root.addResource('{proxy+}');
        proxyResource.addMethod('ANY', lambdaIntegration);
        // Explicit OPTIONS on proxy — forwarded to Lambda CORS middleware
        proxyResource.addMethod('OPTIONS', lambdaIntegration);
        // Add CORS headers to API Gateway's own error responses (4XX/5XX)
        // so browsers can read error details instead of showing opaque CORS errors
        api.addGatewayResponse('Default4XX', {
            type: apigateway.ResponseType.DEFAULT_4XX,
            responseHeaders: {
                'method.response.header.Access-Control-Allow-Origin': "'*'",
                'method.response.header.Access-Control-Allow-Headers': "'Content-Type,Authorization,x-request-id,x-school-id,x-api-key'",
                'method.response.header.Access-Control-Allow-Methods': "'GET,POST,PUT,DELETE,OPTIONS,PATCH'",
            },
        });
        api.addGatewayResponse('Default5XX', {
            type: apigateway.ResponseType.DEFAULT_5XX,
            responseHeaders: {
                'method.response.header.Access-Control-Allow-Origin': "'*'",
                'method.response.header.Access-Control-Allow-Headers': "'Content-Type,Authorization,x-request-id,x-school-id,x-api-key'",
                'method.response.header.Access-Control-Allow-Methods': "'GET,POST,PUT,DELETE,OPTIONS,PATCH'",
            },
        });
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
        new cdk.CfnOutput(this, 'UploadsBucketName', {
            value: uploadsBucket.bucketName,
            description: `${stageName} S3 Uploads Bucket Name`,
            exportName: `Goddard${stageName}UploadsBucketName`,
        });
        new cdk.CfnOutput(this, 'UploadsBucketUrl', {
            value: `https://${uploadsBucket.bucketRegionalDomainName}`,
            description: `${stageName} S3 Uploads Bucket Base URL`,
            exportName: `Goddard${stageName}UploadsBucketUrl`,
        });
    }
}
exports.RustLambdaStack = RustLambdaStack;
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoicnVzdC1sYW1iZGEtc3RhY2suanMiLCJzb3VyY2VSb290IjoiIiwic291cmNlcyI6WyIuLi8uLi9saWIvcnVzdC1sYW1iZGEtc3RhY2sudHMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6Ijs7O0FBQUEsbUNBQW1DO0FBQ25DLGlEQUFpRDtBQUNqRCx5REFBeUQ7QUFDekQsNkNBQTZDO0FBQzdDLHlDQUF5QztBQUN6Qyw2QkFBNkI7QUFPN0IsTUFBYSxlQUFnQixTQUFRLEdBQUcsQ0FBQyxLQUFLO0lBQzVDLFlBQVksS0FBZ0IsRUFBRSxFQUFVLEVBQUUsS0FBdUI7UUFDL0QsS0FBSyxDQUFDLEtBQUssRUFBRSxFQUFFLEVBQUUsS0FBSyxDQUFDLENBQUM7UUFFeEIsTUFBTSxFQUFFLEtBQUssRUFBRSxHQUFHLEtBQUssQ0FBQztRQUN4QixNQUFNLFNBQVMsR0FBRyxLQUFLLENBQUMsV0FBVyxFQUFFLENBQUM7UUFFdEMsc0NBQXNDO1FBQ3RDLE1BQU0sYUFBYSxHQUFHLElBQUksRUFBRSxDQUFDLE1BQU0sQ0FBQyxJQUFJLEVBQUUsVUFBVSxTQUFTLGVBQWUsRUFBRTtZQUM1RSxVQUFVLEVBQUUsbUJBQW1CLEtBQUssRUFBRTtZQUN0QyxnQkFBZ0IsRUFBRSxJQUFJO1lBQ3RCLGlCQUFpQixFQUFFLEVBQUUsQ0FBQyxpQkFBaUIsQ0FBQyxVQUFVO1lBQ2xELElBQUksRUFBRTtnQkFDSjtvQkFDRSxjQUFjLEVBQUUsQ0FBQyxFQUFFLENBQUMsV0FBVyxDQUFDLEdBQUcsRUFBRSxFQUFFLENBQUMsV0FBVyxDQUFDLEdBQUcsQ0FBQztvQkFDeEQsY0FBYyxFQUFFLENBQUMsR0FBRyxDQUFDO29CQUNyQixjQUFjLEVBQUUsQ0FBQyxHQUFHLENBQUM7aUJBQ3RCO2FBQ0Y7WUFDRCxhQUFhLEVBQUUsR0FBRyxDQUFDLGFBQWEsQ0FBQyxNQUFNO1NBQ3hDLENBQUMsQ0FBQztRQUVILGdDQUFnQztRQUNoQyw2RkFBNkY7UUFDN0Ysa0hBQWtIO1FBQ2xILE1BQU0sVUFBVSxHQUFHLElBQUksTUFBTSxDQUFDLFFBQVEsQ0FBQyxJQUFJLEVBQUUsVUFBVSxTQUFTLFFBQVEsRUFBRTtZQUN4RSxZQUFZLEVBQUUsV0FBVyxLQUFLLEVBQUU7WUFDaEMsT0FBTyxFQUFFLE1BQU0sQ0FBQyxPQUFPLENBQUMsZUFBZSxFQUFFLG1DQUFtQztZQUM1RSxZQUFZLEVBQUUsTUFBTSxDQUFDLFlBQVksQ0FBQyxNQUFNLEVBQUUsa0NBQWtDO1lBQzVFLE9BQU8sRUFBRSxXQUFXO1lBQ3BCLElBQUksRUFBRSxNQUFNLENBQUMsSUFBSSxDQUFDLFNBQVMsQ0FBQyxJQUFJLENBQUMsSUFBSSxDQUFDLFNBQVMsRUFBRSxvREFBb0QsQ0FBQyxFQUFFO2dCQUN0RyxPQUFPLEVBQUUsQ0FBQyxJQUFJLEVBQUUsWUFBWSxDQUFDO2FBQzlCLENBQUM7WUFDRixVQUFVLEVBQUUsS0FBSyxLQUFLLEtBQUssQ0FBQyxDQUFDLENBQUMsR0FBRyxDQUFDLENBQUMsQ0FBQyxHQUFHO1lBQ3ZDLE9BQU8sRUFBRSxHQUFHLENBQUMsUUFBUSxDQUFDLE9BQU8sQ0FBQyxFQUFFLENBQUM7WUFDakMsV0FBVyxFQUFFO2dCQUNYLFFBQVEsRUFBRSxNQUFNO2dCQUNoQixnQkFBZ0IsRUFBRSxhQUFhLENBQUMsVUFBVTtnQkFDMUMsV0FBVyxFQUFFLFdBQVcsYUFBYSxDQUFDLHdCQUF3QixFQUFFO2FBQ2pFO1lBQ0QsUUFBUSxFQUFFLElBQUksSUFBSSxDQUFDLFFBQVEsQ0FBQyxJQUFJLEVBQUUsVUFBVSxTQUFTLGdCQUFnQixFQUFFO2dCQUNyRSxZQUFZLEVBQUUsdUJBQXVCLEtBQUssRUFBRTtnQkFDNUMsU0FBUyxFQUFFLElBQUksQ0FBQyxhQUFhLENBQUMsUUFBUTtnQkFDdEMsYUFBYSxFQUFFLEdBQUcsQ0FBQyxhQUFhLENBQUMsT0FBTzthQUN6QyxDQUFDO1lBQ0YsV0FBVyxFQUFFLFdBQVcsU0FBUywrQ0FBK0M7U0FDakYsQ0FBQyxDQUFDO1FBRUgsa0RBQWtEO1FBQ2xELGFBQWEsQ0FBQyxRQUFRLENBQUMsVUFBVSxDQUFDLENBQUM7UUFFbkMsY0FBYztRQUNkLE1BQU0sR0FBRyxHQUFHLElBQUksVUFBVSxDQUFDLE9BQU8sQ0FBQyxJQUFJLEVBQUUsVUFBVSxTQUFTLEtBQUssRUFBRTtZQUNqRSxXQUFXLEVBQUUsV0FBVyxTQUFTLE1BQU07WUFDdkMsV0FBVyxFQUFFLEdBQUcsU0FBUyxrREFBa0Q7WUFDM0UsZ0JBQWdCLEVBQUUsQ0FBQyxLQUFLLENBQUM7WUFDekIsYUFBYSxFQUFFO2dCQUNiLFNBQVMsRUFBRSxLQUFLO2dCQUNoQixjQUFjLEVBQUUsS0FBSyxLQUFLLE1BQU07Z0JBQ2hDLGNBQWMsRUFBRSxJQUFJO2FBQ3JCO1lBQ0QsMkRBQTJEO1lBQzNELGtFQUFrRTtZQUNsRSx5RUFBeUU7WUFDekUsOEVBQThFO1NBQy9FLENBQUMsQ0FBQztRQUVILGdDQUFnQztRQUNoQyxNQUFNLGlCQUFpQixHQUFHLElBQUksVUFBVSxDQUFDLGlCQUFpQixDQUFDLFVBQVUsRUFBRTtZQUNyRSxLQUFLLEVBQUUsSUFBSTtTQUNaLENBQUMsQ0FBQztRQUVILG1CQUFtQjtRQUNuQixHQUFHLENBQUMsSUFBSSxDQUFDLFNBQVMsQ0FBQyxLQUFLLEVBQUUsaUJBQWlCLENBQUMsQ0FBQztRQUM3QyxzRUFBc0U7UUFDdEUsR0FBRyxDQUFDLElBQUksQ0FBQyxTQUFTLENBQUMsU0FBUyxFQUFFLGlCQUFpQixDQUFDLENBQUM7UUFFakQsNENBQTRDO1FBQzVDLE1BQU0sYUFBYSxHQUFHLEdBQUcsQ0FBQyxJQUFJLENBQUMsV0FBVyxDQUFDLFVBQVUsQ0FBQyxDQUFDO1FBQ3ZELGFBQWEsQ0FBQyxTQUFTLENBQUMsS0FBSyxFQUFFLGlCQUFpQixDQUFDLENBQUM7UUFDbEQsa0VBQWtFO1FBQ2xFLGFBQWEsQ0FBQyxTQUFTLENBQUMsU0FBUyxFQUFFLGlCQUFpQixDQUFDLENBQUM7UUFFdEQsa0VBQWtFO1FBQ2xFLDJFQUEyRTtRQUMzRSxHQUFHLENBQUMsa0JBQWtCLENBQUMsWUFBWSxFQUFFO1lBQ25DLElBQUksRUFBRSxVQUFVLENBQUMsWUFBWSxDQUFDLFdBQVc7WUFDekMsZUFBZSxFQUFFO2dCQUNmLG9EQUFvRCxFQUFFLEtBQUs7Z0JBQzNELHFEQUFxRCxFQUFFLGlFQUFpRTtnQkFDeEgscURBQXFELEVBQUUscUNBQXFDO2FBQzdGO1NBQ0YsQ0FBQyxDQUFDO1FBQ0gsR0FBRyxDQUFDLGtCQUFrQixDQUFDLFlBQVksRUFBRTtZQUNuQyxJQUFJLEVBQUUsVUFBVSxDQUFDLFlBQVksQ0FBQyxXQUFXO1lBQ3pDLGVBQWUsRUFBRTtnQkFDZixvREFBb0QsRUFBRSxLQUFLO2dCQUMzRCxxREFBcUQsRUFBRSxpRUFBaUU7Z0JBQ3hILHFEQUFxRCxFQUFFLHFDQUFxQzthQUM3RjtTQUNGLENBQUMsQ0FBQztRQUVILFVBQVU7UUFDVixJQUFJLEdBQUcsQ0FBQyxTQUFTLENBQUMsSUFBSSxFQUFFLFFBQVEsRUFBRTtZQUNoQyxLQUFLLEVBQUUsR0FBRyxDQUFDLEdBQUc7WUFDZCxXQUFXLEVBQUUsR0FBRyxTQUFTLGtCQUFrQjtZQUMzQyxVQUFVLEVBQUUsVUFBVSxTQUFTLFFBQVE7U0FDeEMsQ0FBQyxDQUFDO1FBRUgsSUFBSSxHQUFHLENBQUMsU0FBUyxDQUFDLElBQUksRUFBRSxvQkFBb0IsRUFBRTtZQUM1QyxLQUFLLEVBQUUsVUFBVSxDQUFDLFlBQVk7WUFDOUIsV0FBVyxFQUFFLEdBQUcsU0FBUyx1QkFBdUI7WUFDaEQsVUFBVSxFQUFFLFVBQVUsU0FBUyxvQkFBb0I7U0FDcEQsQ0FBQyxDQUFDO1FBRUgsSUFBSSxHQUFHLENBQUMsU0FBUyxDQUFDLElBQUksRUFBRSxtQkFBbUIsRUFBRTtZQUMzQyxLQUFLLEVBQUUsVUFBVSxDQUFDLFdBQVc7WUFDN0IsV0FBVyxFQUFFLEdBQUcsU0FBUyxzQkFBc0I7WUFDL0MsVUFBVSxFQUFFLFVBQVUsU0FBUyxtQkFBbUI7U0FDbkQsQ0FBQyxDQUFDO1FBRUgsSUFBSSxHQUFHLENBQUMsU0FBUyxDQUFDLElBQUksRUFBRSxtQkFBbUIsRUFBRTtZQUMzQyxLQUFLLEVBQUUsYUFBYSxDQUFDLFVBQVU7WUFDL0IsV0FBVyxFQUFFLEdBQUcsU0FBUyx5QkFBeUI7WUFDbEQsVUFBVSxFQUFFLFVBQVUsU0FBUyxtQkFBbUI7U0FDbkQsQ0FBQyxDQUFDO1FBRUgsSUFBSSxHQUFHLENBQUMsU0FBUyxDQUFDLElBQUksRUFBRSxrQkFBa0IsRUFBRTtZQUMxQyxLQUFLLEVBQUUsV0FBVyxhQUFhLENBQUMsd0JBQXdCLEVBQUU7WUFDMUQsV0FBVyxFQUFFLEdBQUcsU0FBUyw2QkFBNkI7WUFDdEQsVUFBVSxFQUFFLFVBQVUsU0FBUyxrQkFBa0I7U0FDbEQsQ0FBQyxDQUFDO0lBQ0wsQ0FBQztDQUNGO0FBcklELDBDQXFJQyIsInNvdXJjZXNDb250ZW50IjpbImltcG9ydCAqIGFzIGNkayBmcm9tICdhd3MtY2RrLWxpYic7XG5pbXBvcnQgKiBhcyBsYW1iZGEgZnJvbSAnYXdzLWNkay1saWIvYXdzLWxhbWJkYSc7XG5pbXBvcnQgKiBhcyBhcGlnYXRld2F5IGZyb20gJ2F3cy1jZGstbGliL2F3cy1hcGlnYXRld2F5JztcbmltcG9ydCAqIGFzIGxvZ3MgZnJvbSAnYXdzLWNkay1saWIvYXdzLWxvZ3MnO1xuaW1wb3J0ICogYXMgczMgZnJvbSAnYXdzLWNkay1saWIvYXdzLXMzJztcbmltcG9ydCAqIGFzIHBhdGggZnJvbSAncGF0aCc7XG5pbXBvcnQgeyBDb25zdHJ1Y3QgfSBmcm9tICdjb25zdHJ1Y3RzJztcblxuaW50ZXJmYWNlIEdvZGRhclN0YWNrUHJvcHMgZXh0ZW5kcyBjZGsuU3RhY2tQcm9wcyB7XG4gIHN0YWdlOiAnZGV2JyB8ICdwcm9kJztcbn1cblxuZXhwb3J0IGNsYXNzIFJ1c3RMYW1iZGFTdGFjayBleHRlbmRzIGNkay5TdGFjayB7XG4gIGNvbnN0cnVjdG9yKHNjb3BlOiBDb25zdHJ1Y3QsIGlkOiBzdHJpbmcsIHByb3BzOiBHb2RkYXJTdGFja1Byb3BzKSB7XG4gICAgc3VwZXIoc2NvcGUsIGlkLCBwcm9wcyk7XG5cbiAgICBjb25zdCB7IHN0YWdlIH0gPSBwcm9wcztcbiAgICBjb25zdCBzdGFnZU5hbWUgPSBzdGFnZS50b1VwcGVyQ2FzZSgpO1xuXG4gICAgLy8gUzMgYnVja2V0IGZvciBwcm9kdWN0IGltYWdlIHVwbG9hZHNcbiAgICBjb25zdCB1cGxvYWRzQnVja2V0ID0gbmV3IHMzLkJ1Y2tldCh0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfVVwbG9hZHNCdWNrZXRgLCB7XG4gICAgICBidWNrZXROYW1lOiBgZ29kZGFyZC11cGxvYWRzLSR7c3RhZ2V9YCxcbiAgICAgIHB1YmxpY1JlYWRBY2Nlc3M6IHRydWUsXG4gICAgICBibG9ja1B1YmxpY0FjY2VzczogczMuQmxvY2tQdWJsaWNBY2Nlc3MuQkxPQ0tfQUNMUyxcbiAgICAgIGNvcnM6IFtcbiAgICAgICAge1xuICAgICAgICAgIGFsbG93ZWRNZXRob2RzOiBbczMuSHR0cE1ldGhvZHMuR0VULCBzMy5IdHRwTWV0aG9kcy5QVVRdLFxuICAgICAgICAgIGFsbG93ZWRPcmlnaW5zOiBbJyonXSxcbiAgICAgICAgICBhbGxvd2VkSGVhZGVyczogWycqJ10sXG4gICAgICAgIH0sXG4gICAgICBdLFxuICAgICAgcmVtb3ZhbFBvbGljeTogY2RrLlJlbW92YWxQb2xpY3kuUkVUQUlOLFxuICAgIH0pO1xuXG4gICAgLy8gTGFtYmRhIGZ1bmN0aW9uIGZvciBSdXN0IGNvZGVcbiAgICAvLyBVc2luZyBBUk02NCBhcmNoaXRlY3R1cmUgZm9yIHVwIHRvIDM0JSBiZXR0ZXIgcHJpY2UgcGVyZm9ybWFuY2UgYW5kIDE5JSBiZXR0ZXIgcGVyZm9ybWFuY2VcbiAgICAvLyBTZWU6IGh0dHBzOi8vYXdzLmFtYXpvbi5jb20vYmxvZ3MvY29tcHV0ZS9taWdyYXRpbmctYXdzLWxhbWJkYS1mdW5jdGlvbnMtdG8tYXJtLWJhc2VkLWF3cy1ncmF2aXRvbjItcHJvY2Vzc29ycy9cbiAgICBjb25zdCBydXN0TGFtYmRhID0gbmV3IGxhbWJkYS5GdW5jdGlvbih0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfUxhbWJkYWAsIHtcbiAgICAgIGZ1bmN0aW9uTmFtZTogYGdvZGRhcmQtJHtzdGFnZX1gLFxuICAgICAgcnVudGltZTogbGFtYmRhLlJ1bnRpbWUuUFJPVklERURfQUwyMDIzLCAvLyBBbWF6b24gTGludXggMjAyMyBzdXBwb3J0cyBBUk02NFxuICAgICAgYXJjaGl0ZWN0dXJlOiBsYW1iZGEuQXJjaGl0ZWN0dXJlLkFSTV82NCwgLy8gQVdTIEdyYXZpdG9uMiBwcm9jZXNzb3IgKEFSTTY0KVxuICAgICAgaGFuZGxlcjogJ2Jvb3RzdHJhcCcsXG4gICAgICBjb2RlOiBsYW1iZGEuQ29kZS5mcm9tQXNzZXQocGF0aC5qb2luKF9fZGlybmFtZSwgJy4uLy4uL2xhbWJkYS9nb2RkYXJkL3RhcmdldC9sYW1iZGEvZ29kZGFyZC1iYWNrZW5kJyksIHtcbiAgICAgICAgZXhjbHVkZTogWycqKicsICchYm9vdHN0cmFwJ10sXG4gICAgICB9KSxcbiAgICAgIG1lbW9yeVNpemU6IHN0YWdlID09PSAnZGV2JyA/IDEyOCA6IDI1NixcbiAgICAgIHRpbWVvdXQ6IGNkay5EdXJhdGlvbi5zZWNvbmRzKDMwKSxcbiAgICAgIGVudmlyb25tZW50OiB7XG4gICAgICAgIFJVU1RfTE9HOiAnaW5mbycsXG4gICAgICAgIFMzX1VQTE9BRF9CVUNLRVQ6IHVwbG9hZHNCdWNrZXQuYnVja2V0TmFtZSxcbiAgICAgICAgUzNfQkFTRV9VUkw6IGBodHRwczovLyR7dXBsb2Fkc0J1Y2tldC5idWNrZXRSZWdpb25hbERvbWFpbk5hbWV9YCxcbiAgICAgIH0sXG4gICAgICBsb2dHcm91cDogbmV3IGxvZ3MuTG9nR3JvdXAodGhpcywgYEdvZGRhcmQke3N0YWdlTmFtZX1MYW1iZGFMb2dHcm91cGAsIHtcbiAgICAgICAgbG9nR3JvdXBOYW1lOiBgL2F3cy9sYW1iZGEvZ29kZGFyZC0ke3N0YWdlfWAsXG4gICAgICAgIHJldGVudGlvbjogbG9ncy5SZXRlbnRpb25EYXlzLk9ORV9XRUVLLFxuICAgICAgICByZW1vdmFsUG9saWN5OiBjZGsuUmVtb3ZhbFBvbGljeS5ERVNUUk9ZLFxuICAgICAgfSksXG4gICAgICBkZXNjcmlwdGlvbjogYEdvZGRhcmQgJHtzdGFnZU5hbWV9IC0gQmFja2VuZCBMYW1iZGEgZnVuY3Rpb24gd2l0aCBBUEkgZW5kcG9pbnRzYCxcbiAgICB9KTtcblxuICAgIC8vIEdyYW50IExhbWJkYSB3cml0ZSBhY2Nlc3MgdG8gdGhlIHVwbG9hZHMgYnVja2V0XG4gICAgdXBsb2Fkc0J1Y2tldC5ncmFudFB1dChydXN0TGFtYmRhKTtcblxuICAgIC8vIEFQSSBHYXRld2F5XG4gICAgY29uc3QgYXBpID0gbmV3IGFwaWdhdGV3YXkuUmVzdEFwaSh0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfUFwaWAsIHtcbiAgICAgIHJlc3RBcGlOYW1lOiBgR29kZGFyZCAke3N0YWdlTmFtZX0gQVBJYCxcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IEFQSSBHYXRld2F5IGZvciBHb2RkYXJkIEJhY2tlbmQgTGFtYmRhIGZ1bmN0aW9uYCxcbiAgICAgIGJpbmFyeU1lZGlhVHlwZXM6IFsnKi8qJ10sXG4gICAgICBkZXBsb3lPcHRpb25zOiB7XG4gICAgICAgIHN0YWdlTmFtZTogc3RhZ2UsXG4gICAgICAgIHRyYWNpbmdFbmFibGVkOiBzdGFnZSA9PT0gJ3Byb2QnLFxuICAgICAgICBtZXRyaWNzRW5hYmxlZDogdHJ1ZSxcbiAgICAgIH0sXG4gICAgICAvLyBDT1JTIGlzIGhhbmRsZWQgZW50aXJlbHkgYnkgTGFtYmRhIG1pZGRsZXdhcmUgKGNvcnMucnMpLlxuICAgICAgLy8gRG8gTk9UIHVzZSBkZWZhdWx0Q29yc1ByZWZsaWdodE9wdGlvbnMgaGVyZSDigJQgaXQgY3JlYXRlcyBhIE1PQ0tcbiAgICAgIC8vIGludGVncmF0aW9uIGZvciBPUFRJT05TIHRoYXQgY29uZmxpY3RzIHdpdGggYmluYXJ5TWVkaWFUeXBlczogWycqLyonXSxcbiAgICAgIC8vIGNhdXNpbmcgQVBJIEdhdGV3YXkgdG8gY29ycnVwdC9zdHJpcCBDT1JTIGhlYWRlcnMgZnJvbSBwcmVmbGlnaHQgcmVzcG9uc2VzLlxuICAgIH0pO1xuXG4gICAgLy8gTGFtYmRhIGludGVncmF0aW9uIHdpdGggcHJveHlcbiAgICBjb25zdCBsYW1iZGFJbnRlZ3JhdGlvbiA9IG5ldyBhcGlnYXRld2F5LkxhbWJkYUludGVncmF0aW9uKHJ1c3RMYW1iZGEsIHtcbiAgICAgIHByb3h5OiB0cnVlLFxuICAgIH0pO1xuXG4gICAgLy8gSGFuZGxlIHJvb3QgcGF0aFxuICAgIGFwaS5yb290LmFkZE1ldGhvZCgnQU5ZJywgbGFtYmRhSW50ZWdyYXRpb24pO1xuICAgIC8vIEV4cGxpY2l0IE9QVElPTlMgb24gcm9vdCDigJQgQU5ZIGRvZXMgTk9UIGZvcndhcmQgT1BUSU9OUyBpbiBSRVNUIEFQSVxuICAgIGFwaS5yb290LmFkZE1ldGhvZCgnT1BUSU9OUycsIGxhbWJkYUludGVncmF0aW9uKTtcblxuICAgIC8vIENyZWF0ZSBwcm94eSByZXNvdXJjZSBmb3IgYWxsIG90aGVyIHBhdGhzXG4gICAgY29uc3QgcHJveHlSZXNvdXJjZSA9IGFwaS5yb290LmFkZFJlc291cmNlKCd7cHJveHkrfScpO1xuICAgIHByb3h5UmVzb3VyY2UuYWRkTWV0aG9kKCdBTlknLCBsYW1iZGFJbnRlZ3JhdGlvbik7XG4gICAgLy8gRXhwbGljaXQgT1BUSU9OUyBvbiBwcm94eSDigJQgZm9yd2FyZGVkIHRvIExhbWJkYSBDT1JTIG1pZGRsZXdhcmVcbiAgICBwcm94eVJlc291cmNlLmFkZE1ldGhvZCgnT1BUSU9OUycsIGxhbWJkYUludGVncmF0aW9uKTtcblxuICAgIC8vIEFkZCBDT1JTIGhlYWRlcnMgdG8gQVBJIEdhdGV3YXkncyBvd24gZXJyb3IgcmVzcG9uc2VzICg0WFgvNVhYKVxuICAgIC8vIHNvIGJyb3dzZXJzIGNhbiByZWFkIGVycm9yIGRldGFpbHMgaW5zdGVhZCBvZiBzaG93aW5nIG9wYXF1ZSBDT1JTIGVycm9yc1xuICAgIGFwaS5hZGRHYXRld2F5UmVzcG9uc2UoJ0RlZmF1bHQ0WFgnLCB7XG4gICAgICB0eXBlOiBhcGlnYXRld2F5LlJlc3BvbnNlVHlwZS5ERUZBVUxUXzRYWCxcbiAgICAgIHJlc3BvbnNlSGVhZGVyczoge1xuICAgICAgICAnbWV0aG9kLnJlc3BvbnNlLmhlYWRlci5BY2Nlc3MtQ29udHJvbC1BbGxvdy1PcmlnaW4nOiBcIicqJ1wiLFxuICAgICAgICAnbWV0aG9kLnJlc3BvbnNlLmhlYWRlci5BY2Nlc3MtQ29udHJvbC1BbGxvdy1IZWFkZXJzJzogXCInQ29udGVudC1UeXBlLEF1dGhvcml6YXRpb24seC1yZXF1ZXN0LWlkLHgtc2Nob29sLWlkLHgtYXBpLWtleSdcIixcbiAgICAgICAgJ21ldGhvZC5yZXNwb25zZS5oZWFkZXIuQWNjZXNzLUNvbnRyb2wtQWxsb3ctTWV0aG9kcyc6IFwiJ0dFVCxQT1NULFBVVCxERUxFVEUsT1BUSU9OUyxQQVRDSCdcIixcbiAgICAgIH0sXG4gICAgfSk7XG4gICAgYXBpLmFkZEdhdGV3YXlSZXNwb25zZSgnRGVmYXVsdDVYWCcsIHtcbiAgICAgIHR5cGU6IGFwaWdhdGV3YXkuUmVzcG9uc2VUeXBlLkRFRkFVTFRfNVhYLFxuICAgICAgcmVzcG9uc2VIZWFkZXJzOiB7XG4gICAgICAgICdtZXRob2QucmVzcG9uc2UuaGVhZGVyLkFjY2Vzcy1Db250cm9sLUFsbG93LU9yaWdpbic6IFwiJyonXCIsXG4gICAgICAgICdtZXRob2QucmVzcG9uc2UuaGVhZGVyLkFjY2Vzcy1Db250cm9sLUFsbG93LUhlYWRlcnMnOiBcIidDb250ZW50LVR5cGUsQXV0aG9yaXphdGlvbix4LXJlcXVlc3QtaWQseC1zY2hvb2wtaWQseC1hcGkta2V5J1wiLFxuICAgICAgICAnbWV0aG9kLnJlc3BvbnNlLmhlYWRlci5BY2Nlc3MtQ29udHJvbC1BbGxvdy1NZXRob2RzJzogXCInR0VULFBPU1QsUFVULERFTEVURSxPUFRJT05TLFBBVENIJ1wiLFxuICAgICAgfSxcbiAgICB9KTtcblxuICAgIC8vIE91dHB1dHNcbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnQXBpVXJsJywge1xuICAgICAgdmFsdWU6IGFwaS51cmwsXG4gICAgICBkZXNjcmlwdGlvbjogYCR7c3RhZ2VOYW1lfSBBUEkgR2F0ZXdheSBVUkxgLFxuICAgICAgZXhwb3J0TmFtZTogYEdvZGRhcmQke3N0YWdlTmFtZX1BcGlVcmxgLFxuICAgIH0pO1xuXG4gICAgbmV3IGNkay5DZm5PdXRwdXQodGhpcywgJ0xhbWJkYUZ1bmN0aW9uTmFtZScsIHtcbiAgICAgIHZhbHVlOiBydXN0TGFtYmRhLmZ1bmN0aW9uTmFtZSxcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IExhbWJkYSBGdW5jdGlvbiBOYW1lYCxcbiAgICAgIGV4cG9ydE5hbWU6IGBHb2RkYXJkJHtzdGFnZU5hbWV9TGFtYmRhRnVuY3Rpb25OYW1lYCxcbiAgICB9KTtcblxuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdMYW1iZGFGdW5jdGlvbkFybicsIHtcbiAgICAgIHZhbHVlOiBydXN0TGFtYmRhLmZ1bmN0aW9uQXJuLFxuICAgICAgZGVzY3JpcHRpb246IGAke3N0YWdlTmFtZX0gTGFtYmRhIEZ1bmN0aW9uIEFSTmAsXG4gICAgICBleHBvcnROYW1lOiBgR29kZGFyZCR7c3RhZ2VOYW1lfUxhbWJkYUZ1bmN0aW9uQXJuYCxcbiAgICB9KTtcblxuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdVcGxvYWRzQnVja2V0TmFtZScsIHtcbiAgICAgIHZhbHVlOiB1cGxvYWRzQnVja2V0LmJ1Y2tldE5hbWUsXG4gICAgICBkZXNjcmlwdGlvbjogYCR7c3RhZ2VOYW1lfSBTMyBVcGxvYWRzIEJ1Y2tldCBOYW1lYCxcbiAgICAgIGV4cG9ydE5hbWU6IGBHb2RkYXJkJHtzdGFnZU5hbWV9VXBsb2Fkc0J1Y2tldE5hbWVgLFxuICAgIH0pO1xuXG4gICAgbmV3IGNkay5DZm5PdXRwdXQodGhpcywgJ1VwbG9hZHNCdWNrZXRVcmwnLCB7XG4gICAgICB2YWx1ZTogYGh0dHBzOi8vJHt1cGxvYWRzQnVja2V0LmJ1Y2tldFJlZ2lvbmFsRG9tYWluTmFtZX1gLFxuICAgICAgZGVzY3JpcHRpb246IGAke3N0YWdlTmFtZX0gUzMgVXBsb2FkcyBCdWNrZXQgQmFzZSBVUkxgLFxuICAgICAgZXhwb3J0TmFtZTogYEdvZGRhcmQke3N0YWdlTmFtZX1VcGxvYWRzQnVja2V0VXJsYCxcbiAgICB9KTtcbiAgfVxufVxuIl19