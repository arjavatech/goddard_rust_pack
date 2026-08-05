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
    }
}
exports.RustLambdaStack = RustLambdaStack;
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoicnVzdC1sYW1iZGEtc3RhY2suanMiLCJzb3VyY2VSb290IjoiIiwic291cmNlcyI6WyIuLi8uLi9saWIvcnVzdC1sYW1iZGEtc3RhY2sudHMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6Ijs7O0FBQUEsbUNBQW1DO0FBQ25DLGlEQUFpRDtBQUNqRCx5REFBeUQ7QUFDekQsNkNBQTZDO0FBQzdDLDZCQUE2QjtBQU83QixNQUFhLGVBQWdCLFNBQVEsR0FBRyxDQUFDLEtBQUs7SUFDNUMsWUFBWSxLQUFnQixFQUFFLEVBQVUsRUFBRSxLQUF1QjtRQUMvRCxLQUFLLENBQUMsS0FBSyxFQUFFLEVBQUUsRUFBRSxLQUFLLENBQUMsQ0FBQztRQUV4QixNQUFNLEVBQUUsS0FBSyxFQUFFLEdBQUcsS0FBSyxDQUFDO1FBQ3hCLE1BQU0sU0FBUyxHQUFHLEtBQUssQ0FBQyxXQUFXLEVBQUUsQ0FBQztRQUV0QyxnQ0FBZ0M7UUFDaEMsNkZBQTZGO1FBQzdGLGtIQUFrSDtRQUNsSCxNQUFNLFVBQVUsR0FBRyxJQUFJLE1BQU0sQ0FBQyxRQUFRLENBQUMsSUFBSSxFQUFFLFVBQVUsU0FBUyxRQUFRLEVBQUU7WUFDeEUsWUFBWSxFQUFFLFdBQVcsS0FBSyxFQUFFO1lBQ2hDLE9BQU8sRUFBRSxNQUFNLENBQUMsT0FBTyxDQUFDLGVBQWUsRUFBRSxtQ0FBbUM7WUFDNUUsWUFBWSxFQUFFLE1BQU0sQ0FBQyxZQUFZLENBQUMsTUFBTSxFQUFFLGtDQUFrQztZQUM1RSxPQUFPLEVBQUUsV0FBVztZQUNwQixJQUFJLEVBQUUsTUFBTSxDQUFDLElBQUksQ0FBQyxTQUFTLENBQUMsSUFBSSxDQUFDLElBQUksQ0FBQyxTQUFTLEVBQUUsb0RBQW9ELENBQUMsRUFBRTtnQkFDdEcsT0FBTyxFQUFFLENBQUMsSUFBSSxFQUFFLFlBQVksQ0FBQzthQUM5QixDQUFDO1lBQ0YsVUFBVSxFQUFFLEtBQUssS0FBSyxLQUFLLENBQUMsQ0FBQyxDQUFDLEdBQUcsQ0FBQyxDQUFDLENBQUMsR0FBRztZQUN2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QyxZQUFZO1lBQzlCLFdBQVcsRUFBRSxHQUFHLFNBQVMsdUJBQXVCO1lBQ2hELFVBQVUsRUFBRSxVQUFVLFNBQVMsb0JBQW9CO1NBQ3BELENBQUMsQ0FBQztRQUVILElBQUksR0FBRyxDQUFDLFNBQVMsQ0FBQyxJQUFJLEVBQUUsbUJBQW1CLEVBQUU7WUFDM0MsS0FBSyxFQUFFLFVBQVUsQ0FBQyxXQUFXO1lBQzdCLFdBQVcsRUFBRSxHQUFHLFNBQVMsc0JBQXNCO1lBQy9DLFVBQVUsRUFBRSxVQUFVLFNBQVMsbUJBQW1CO1NBQ25ELENBQUMsQ0FBQztJQUNMLENBQUM7Q0FDRjtBQXJHRCwwQ0FxR0MiLCJzb3VyY2VzQ29udGVudCI6WyJpbXBvcnQgKiBhcyBjZGsgZnJvbSAnYXdzLWNkay1saWInO1xuaW1wb3J0ICogYXMgbGFtYmRhIGZyb20gJ2F3cy1jZGstbGliL2F3cy1sYW1iZGEnO1xuaW1wb3J0ICogYXMgYXBpZ2F0ZXdheSBmcm9tICdhd3MtY2RrLWxpYi9hd3MtYXBpZ2F0ZXdheSc7XG5pbXBvcnQgKiBhcyBsb2dzIGZyb20gJ2F3cy1jZGstbGliL2F3cy1sb2dzJztcbmltcG9ydCAqIGFzIHBhdGggZnJvbSAncGF0aCc7XG5pbXBvcnQgeyBDb25zdHJ1Y3QgfSBmcm9tICdjb25zdHJ1Y3RzJztcblxuaW50ZXJmYWNlIEdvZGRhclN0YWNrUHJvcHMgZXh0ZW5kcyBjZGsuU3RhY2tQcm9wcyB7XG4gIHN0YWdlOiAnZGV2JyB8ICdwcm9kJztcbn1cblxuZXhwb3J0IGNsYXNzIFJ1c3RMYW1iZGFTdGFjayBleHRlbmRzIGNkay5TdGFjayB7XG4gIGNvbnN0cnVjdG9yKHNjb3BlOiBDb25zdHJ1Y3QsIGlkOiBzdHJpbmcsIHByb3BzOiBHb2RkYXJTdGFja1Byb3BzKSB7XG4gICAgc3VwZXIoc2NvcGUsIGlkLCBwcm9wcyk7XG5cbiAgICBjb25zdCB7IHN0YWdlIH0gPSBwcm9wcztcbiAgICBjb25zdCBzdGFnZU5hbWUgPSBzdGFnZS50b1VwcGVyQ2FzZSgpO1xuXG4gICAgLy8gTGFtYmRhIGZ1bmN0aW9uIGZvciBSdXN0IGNvZGVcbiAgICAvLyBVc2luZyBBUk02NCBhcmNoaXRlY3R1cmUgZm9yIHVwIHRvIDM0JSBiZXR0ZXIgcHJpY2UgcGVyZm9ybWFuY2UgYW5kIDE5JSBiZXR0ZXIgcGVyZm9ybWFuY2VcbiAgICAvLyBTZWU6IGh0dHBzOi8vYXdzLmFtYXpvbi5jb20vYmxvZ3MvY29tcHV0ZS9taWdyYXRpbmctYXdzLWxhbWJkYS1mdW5jdGlvbnMtdG8tYXJtLWJhc2VkLWF3cy1ncmF2aXRvbjItcHJvY2Vzc29ycy9cbiAgICBjb25zdCBydXN0TGFtYmRhID0gbmV3IGxhbWJkYS5GdW5jdGlvbih0aGlzLCBgR29kZGFyZCR7c3RhZ2VOYW1lfUxhbWJkYWAsIHtcbiAgICAgIGZ1bmN0aW9uTmFtZTogYGdvZGRhcmQtJHtzdGFnZX1gLFxuICAgICAgcnVudGltZTogbGFtYmRhLlJ1bnRpbWUuUFJPVklERURfQUwyMDIzLCAvLyBBbWF6b24gTGludXggMjAyMyBzdXBwb3J0cyBBUk02NFxuICAgICAgYXJjaGl0ZWN0dXJlOiBsYW1iZGEuQXJjaGl0ZWN0dXJlLkFSTV82NCwgLy8gQVdTIEdyYXZpdG9uMiBwcm9jZXNzb3IgKEFSTTY0KVxuICAgICAgaGFuZGxlcjogJ2Jvb3RzdHJhcCcsXG4gICAgICBjb2RlOiBsYW1iZGEuQ29kZS5mcm9tQXNzZXQocGF0aC5qb2luKF9fZGlybmFtZSwgJy4uLy4uL2xhbWJkYS9nb2RkYXJkL3RhcmdldC9sYW1iZGEvZ29kZGFyZC1iYWNrZW5kJyksIHtcbiAgICAgICAgZXhjbHVkZTogWycqKicsICchYm9vdHN0cmFwJ10sXG4gICAgICB9KSxcbiAgICAgIG1lbW9yeVNpemU6IHN0YWdlID09PSAnZGV2JyA/IDEyOCA6IDI1NixcbiAgICAgIHRpbWVvdXQ6IGNkay5EdXJhdGlvbi5zZWNvbmRzKDMwKSxcbiAgICAgIGVudmlyb25tZW50OiB7XG4gICAgICAgIFJVU1RfTE9HOiAnaW5mbycsXG4gICAgICB9LFxuICAgICAgbG9nR3JvdXA6IG5ldyBsb2dzLkxvZ0dyb3VwKHRoaXMsIGBHb2RkYXJkJHtzdGFnZU5hbWV9TGFtYmRhTG9nR3JvdXBgLCB7XG4gICAgICAgIGxvZ0dyb3VwTmFtZTogYC9hd3MvbGFtYmRhL2dvZGRhcmQtJHtzdGFnZX1gLFxuICAgICAgICByZXRlbnRpb246IGxvZ3MuUmV0ZW50aW9uRGF5cy5PTkVfV0VFSyxcbiAgICAgICAgcmVtb3ZhbFBvbGljeTogY2RrLlJlbW92YWxQb2xpY3kuREVTVFJPWSxcbiAgICAgIH0pLFxuICAgICAgZGVzY3JpcHRpb246IGBHb2RkYXJkICR7c3RhZ2VOYW1lfSAtIEJhY2tlbmQgTGFtYmRhIGZ1bmN0aW9uIHdpdGggQVBJIGVuZHBvaW50c2AsXG4gICAgfSk7XG5cbiAgICAvLyBBUEkgR2F0ZXdheVxuICAgIGNvbnN0IGFwaSA9IG5ldyBhcGlnYXRld2F5LlJlc3RBcGkodGhpcywgYEdvZGRhcmQke3N0YWdlTmFtZX1BcGlgLCB7XG4gICAgICByZXN0QXBpTmFtZTogYEdvZGRhcmQgJHtzdGFnZU5hbWV9IEFQSWAsXG4gICAgICBkZXNjcmlwdGlvbjogYCR7c3RhZ2VOYW1lfSBBUEkgR2F0ZXdheSBmb3IgR29kZGFyZCBCYWNrZW5kIExhbWJkYSBmdW5jdGlvbmAsXG4gICAgICBiaW5hcnlNZWRpYVR5cGVzOiBbJyovKiddLFxuICAgICAgZGVwbG95T3B0aW9uczoge1xuICAgICAgICBzdGFnZU5hbWU6IHN0YWdlLFxuICAgICAgICB0cmFjaW5nRW5hYmxlZDogc3RhZ2UgPT09ICdwcm9kJyxcbiAgICAgICAgbWV0cmljc0VuYWJsZWQ6IHRydWUsXG4gICAgICB9LFxuICAgICAgLy8gQ09SUyBpcyBoYW5kbGVkIGVudGlyZWx5IGJ5IExhbWJkYSBtaWRkbGV3YXJlIChjb3JzLnJzKS5cbiAgICAgIC8vIERvIE5PVCB1c2UgZGVmYXVsdENvcnNQcmVmbGlnaHRPcHRpb25zIGhlcmUg4oCUIGl0IGNyZWF0ZXMgYSBNT0NLXG4gICAgICAvLyBpbnRlZ3JhdGlvbiBmb3IgT1BUSU9OUyB0aGF0IGNvbmZsaWN0cyB3aXRoIGJpbmFyeU1lZGlhVHlwZXM6IFsnKi8qJ10sXG4gICAgICAvLyBjYXVzaW5nIEFQSSBHYXRld2F5IHRvIGNvcnJ1cHQvc3RyaXAgQ09SUyBoZWFkZXJzIGZyb20gcHJlZmxpZ2h0IHJlc3BvbnNlcy5cbiAgICB9KTtcblxuICAgIC8vIExhbWJkYSBpbnRlZ3JhdGlvbiB3aXRoIHByb3h5XG4gICAgY29uc3QgbGFtYmRhSW50ZWdyYXRpb24gPSBuZXcgYXBpZ2F0ZXdheS5MYW1iZGFJbnRlZ3JhdGlvbihydXN0TGFtYmRhLCB7XG4gICAgICBwcm94eTogdHJ1ZSxcbiAgICB9KTtcblxuICAgIC8vIEhhbmRsZSByb290IHBhdGhcbiAgICBhcGkucm9vdC5hZGRNZXRob2QoJ0FOWScsIGxhbWJkYUludGVncmF0aW9uKTtcbiAgICAvLyBFeHBsaWNpdCBPUFRJT05TIG9uIHJvb3Qg4oCUIEFOWSBkb2VzIE5PVCBmb3J3YXJkIE9QVElPTlMgaW4gUkVTVCBBUElcbiAgICBhcGkucm9vdC5hZGRNZXRob2QoJ09QVElPTlMnLCBsYW1iZGFJbnRlZ3JhdGlvbik7XG5cbiAgICAvLyBDcmVhdGUgcHJveHkgcmVzb3VyY2UgZm9yIGFsbCBvdGhlciBwYXRoc1xuICAgIGNvbnN0IHByb3h5UmVzb3VyY2UgPSBhcGkucm9vdC5hZGRSZXNvdXJjZSgne3Byb3h5K30nKTtcbiAgICBwcm94eVJlc291cmNlLmFkZE1ldGhvZCgnQU5ZJywgbGFtYmRhSW50ZWdyYXRpb24pO1xuICAgIC8vIEV4cGxpY2l0IE9QVElPTlMgb24gcHJveHkg4oCUIGZvcndhcmRlZCB0byBMYW1iZGEgQ09SUyBtaWRkbGV3YXJlXG4gICAgcHJveHlSZXNvdXJjZS5hZGRNZXRob2QoJ09QVElPTlMnLCBsYW1iZGFJbnRlZ3JhdGlvbik7XG5cbiAgICAvLyBBZGQgQ09SUyBoZWFkZXJzIHRvIEFQSSBHYXRld2F5J3Mgb3duIGVycm9yIHJlc3BvbnNlcyAoNFhYLzVYWClcbiAgICAvLyBzbyBicm93c2VycyBjYW4gcmVhZCBlcnJvciBkZXRhaWxzIGluc3RlYWQgb2Ygc2hvd2luZyBvcGFxdWUgQ09SUyBlcnJvcnNcbiAgICBhcGkuYWRkR2F0ZXdheVJlc3BvbnNlKCdEZWZhdWx0NFhYJywge1xuICAgICAgdHlwZTogYXBpZ2F0ZXdheS5SZXNwb25zZVR5cGUuREVGQVVMVF80WFgsXG4gICAgICByZXNwb25zZUhlYWRlcnM6IHtcbiAgICAgICAgJ21ldGhvZC5yZXNwb25zZS5oZWFkZXIuQWNjZXNzLUNvbnRyb2wtQWxsb3ctT3JpZ2luJzogXCInKidcIixcbiAgICAgICAgJ21ldGhvZC5yZXNwb25zZS5oZWFkZXIuQWNjZXNzLUNvbnRyb2wtQWxsb3ctSGVhZGVycyc6IFwiJ0NvbnRlbnQtVHlwZSxBdXRob3JpemF0aW9uLHgtcmVxdWVzdC1pZCx4LXNjaG9vbC1pZCx4LWFwaS1rZXknXCIsXG4gICAgICAgICdtZXRob2QucmVzcG9uc2UuaGVhZGVyLkFjY2Vzcy1Db250cm9sLUFsbG93LU1ldGhvZHMnOiBcIidHRVQsUE9TVCxQVVQsREVMRVRFLE9QVElPTlMsUEFUQ0gnXCIsXG4gICAgICB9LFxuICAgIH0pO1xuICAgIGFwaS5hZGRHYXRld2F5UmVzcG9uc2UoJ0RlZmF1bHQ1WFgnLCB7XG4gICAgICB0eXBlOiBhcGlnYXRld2F5LlJlc3BvbnNlVHlwZS5ERUZBVUxUXzVYWCxcbiAgICAgIHJlc3BvbnNlSGVhZGVyczoge1xuICAgICAgICAnbWV0aG9kLnJlc3BvbnNlLmhlYWRlci5BY2Nlc3MtQ29udHJvbC1BbGxvdy1PcmlnaW4nOiBcIicqJ1wiLFxuICAgICAgICAnbWV0aG9kLnJlc3BvbnNlLmhlYWRlci5BY2Nlc3MtQ29udHJvbC1BbGxvdy1IZWFkZXJzJzogXCInQ29udGVudC1UeXBlLEF1dGhvcml6YXRpb24seC1yZXF1ZXN0LWlkLHgtc2Nob29sLWlkLHgtYXBpLWtleSdcIixcbiAgICAgICAgJ21ldGhvZC5yZXNwb25zZS5oZWFkZXIuQWNjZXNzLUNvbnRyb2wtQWxsb3ctTWV0aG9kcyc6IFwiJ0dFVCxQT1NULFBVVCxERUxFVEUsT1BUSU9OUyxQQVRDSCdcIixcbiAgICAgIH0sXG4gICAgfSk7XG5cbiAgICAvLyBPdXRwdXRzXG4gICAgbmV3IGNkay5DZm5PdXRwdXQodGhpcywgJ0FwaVVybCcsIHtcbiAgICAgIHZhbHVlOiBhcGkudXJsLFxuICAgICAgZGVzY3JpcHRpb246IGAke3N0YWdlTmFtZX0gQVBJIEdhdGV3YXkgVVJMYCxcbiAgICAgIGV4cG9ydE5hbWU6IGBHb2RkYXJkJHtzdGFnZU5hbWV9QXBpVXJsYCxcbiAgICB9KTtcblxuICAgIG5ldyBjZGsuQ2ZuT3V0cHV0KHRoaXMsICdMYW1iZGFGdW5jdGlvbk5hbWUnLCB7XG4gICAgICB2YWx1ZTogcnVzdExhbWJkYS5mdW5jdGlvbk5hbWUsXG4gICAgICBkZXNjcmlwdGlvbjogYCR7c3RhZ2VOYW1lfSBMYW1iZGEgRnVuY3Rpb24gTmFtZWAsXG4gICAgICBleHBvcnROYW1lOiBgR29kZGFyZCR7c3RhZ2VOYW1lfUxhbWJkYUZ1bmN0aW9uTmFtZWAsXG4gICAgfSk7XG5cbiAgICBuZXcgY2RrLkNmbk91dHB1dCh0aGlzLCAnTGFtYmRhRnVuY3Rpb25Bcm4nLCB7XG4gICAgICB2YWx1ZTogcnVzdExhbWJkYS5mdW5jdGlvbkFybixcbiAgICAgIGRlc2NyaXB0aW9uOiBgJHtzdGFnZU5hbWV9IExhbWJkYSBGdW5jdGlvbiBBUk5gLFxuICAgICAgZXhwb3J0TmFtZTogYEdvZGRhcmQke3N0YWdlTmFtZX1MYW1iZGFGdW5jdGlvbkFybmAsXG4gICAgfSk7XG4gIH1cbn1cbiJdfQ==