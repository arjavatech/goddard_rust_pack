import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
interface GoddarStackProps extends cdk.StackProps {
    stage: 'dev' | 'prod';
}
export declare class RustLambdaStack extends cdk.Stack {
    constructor(scope: Construct, id: string, props: GoddarStackProps);
}
export {};
