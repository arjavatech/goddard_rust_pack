#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { RustLambdaStack } from '../lib/rust-lambda-stack';

const app = new cdk.App();

new RustLambdaStack(app, 'GoddardDevStack', {
  stage: 'dev',
  env: { account: process.env.CDK_DEFAULT_ACCOUNT, region: 'us-west-1' },
  description: 'Goddard Dev - Rust Lambda API',
});

new RustLambdaStack(app, 'GoddardProdStack', {
  stage: 'prod',
  env: { account: process.env.CDK_DEFAULT_ACCOUNT, region: 'us-west-1' },
  description: 'Goddard Production - Rust Lambda API',
});

app.synth();
