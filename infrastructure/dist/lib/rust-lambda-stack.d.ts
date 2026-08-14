import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
interface GoddarStackProps extends cdk.StackProps {
    stage: 'dev' | 'prod';
}
export declare class RustLambdaStack extends cdk.Stack {
    constructor(scope: Construct, id: string, props: GoddarStackProps);
    /**
     * The database backup is deliberately isolated from the API Lambda. The
     * Supabase CLI starts pg_dump in Docker, which is supported by privileged
     * CodeBuild but not by Lambda.
     */
    private addBackupPipeline;
}
export {};
