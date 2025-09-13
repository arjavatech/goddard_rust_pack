import * as cdk from 'aws-cdk-lib';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as logs from 'aws-cdk-lib/aws-logs';
import * as iam from 'aws-cdk-lib/aws-iam';
import { Construct } from 'constructs';
import * as path from 'path';

export interface RustLambdaConstructProps {
  readonly functionName: string;
  readonly description?: string;
  readonly timeout?: cdk.Duration;
  readonly memorySize?: number;
  readonly environment?: { [key: string]: string };
  readonly reservedConcurrentExecutions?: number;
}

export class RustLambdaConstruct extends Construct {
  public readonly lambdaFunction: lambda.Function;
  public readonly logGroup: logs.LogGroup;

  constructor(scope: Construct, id: string, props: RustLambdaConstructProps) {
    super(scope, id);

    // Create IAM role for Lambda function
    const lambdaRole = new iam.Role(this, 'LambdaExecutionRole', {
      assumedBy: new iam.ServicePrincipal('lambda.amazonaws.com'),
      managedPolicies: [
        iam.ManagedPolicy.fromAwsManagedPolicyName('service-role/AWSLambdaBasicExecutionRole'),
      ],
      description: 'Execution role for Rust Lambda function',
    });

    // Create CloudWatch Log Group
    this.logGroup = new logs.LogGroup(this, 'LambdaLogGroup', {
      logGroupName: `/aws/lambda/${props.functionName}`,
      retention: logs.RetentionDays.ONE_WEEK,
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    // Grant permissions to write to CloudWatch Logs
    this.logGroup.grantWrite(lambdaRole);

    // Create the Lambda function
    this.lambdaFunction = new lambda.Function(this, 'Function', {
      functionName: props.functionName,
      runtime: lambda.Runtime.PROVIDED_AL2023,
      architecture: lambda.Architecture.X86_64,
      handler: 'bootstrap',
      code: lambda.Code.fromAsset(this.getLambdaCodePath()),
      role: lambdaRole,
      description: props.description || 'Rust Lambda function',
      timeout: props.timeout || cdk.Duration.seconds(30),
      memorySize: props.memorySize || 256,
      environment: {
        RUST_LOG: 'info',
        ...props.environment,
      },
      reservedConcurrentExecutions: props.reservedConcurrentExecutions,
      logGroup: this.logGroup,
      tracing: lambda.Tracing.ACTIVE,
    });

    // Add tags to the function
    cdk.Tags.of(this.lambdaFunction).add('Runtime', 'Rust');
    cdk.Tags.of(this.lambdaFunction).add('Framework', 'lambda-web');
  }

  /**
   * Get the path to the Lambda code (built binary)
   */
  private getLambdaCodePath(): string {
    // This assumes the CDK is run from the project root
    // and the Lambda binary is built in ../lambda/target/lambda/rust-hello-world-lambda/
    return path.join(__dirname, '..', '..', '..', 'lambda', 'target', 'lambda', 'hello-world-lambda');
  }

  /**
   * Grant API Gateway permission to invoke this Lambda function
   */
  public grantInvoke(principal: iam.IPrincipal): iam.Grant {
    return this.lambdaFunction.grantInvoke(principal);
  }

  /**
   * Add environment variables to the Lambda function
   */
  public addEnvironment(key: string, value: string): void {
    this.lambdaFunction.addEnvironment(key, value);
  }

  /**
   * Add permissions to the Lambda function's execution role
   */
  public addToRolePolicy(statement: iam.PolicyStatement): void {
    this.lambdaFunction.addToRolePolicy(statement);
  }
}