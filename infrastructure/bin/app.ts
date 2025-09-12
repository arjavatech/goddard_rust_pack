#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { RustLambdaStack } from '../lib/rust-lambda-stack';

const app = new cdk.App();

new RustLambdaStack(app, 'RustLambdaStack', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION,
  },
  description: 'Rust Lambda API deployed with CDK',
});

app.synth();