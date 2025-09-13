#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { RustLambdaStack } from './stacks/rust-lambda-stack';

const app = new cdk.App();

// Get environment configuration
const account = process.env.CDK_DEFAULT_ACCOUNT;
const region = process.env.CDK_DEFAULT_REGION || 'us-east-1';

// Create stack with proper naming and environment
const stackName = 'RustLambdaApiStack';
new RustLambdaStack(app, stackName, {
  env: {
    account,
    region,
  },
  description: 'Rust Lambda API with API Gateway - CDK Stack',
  tags: {
    Project: 'RustLambdaApi',
    Environment: process.env.NODE_ENV || 'development',
    ManagedBy: 'CDK',
  },
});

// Add stack-level tags
cdk.Tags.of(app).add('CreatedBy', 'AWS CDK');
cdk.Tags.of(app).add('Project', 'RustLambdaApi');