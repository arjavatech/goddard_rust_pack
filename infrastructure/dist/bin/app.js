#!/usr/bin/env node
"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
require("source-map-support/register");
const cdk = require("aws-cdk-lib");
const rust_lambda_stack_1 = require("../lib/rust-lambda-stack");
const app = new cdk.App();
new rust_lambda_stack_1.RustLambdaStack(app, 'GoddardProdStack', {
    description: 'Goddard Production - Rust Lambda API deployed with CDK',
    env: {
        region: 'us-west-1',
    },
});
app.synth();
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiYXBwLmpzIiwic291cmNlUm9vdCI6IiIsInNvdXJjZXMiOlsiLi4vLi4vYmluL2FwcC50cyJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiOzs7QUFDQSx1Q0FBcUM7QUFDckMsbUNBQW1DO0FBQ25DLGdFQUEyRDtBQUUzRCxNQUFNLEdBQUcsR0FBRyxJQUFJLEdBQUcsQ0FBQyxHQUFHLEVBQUUsQ0FBQztBQUUxQixJQUFJLG1DQUFlLENBQUMsR0FBRyxFQUFFLGtCQUFrQixFQUFFO0lBQzNDLFdBQVcsRUFBRSx3REFBd0Q7SUFDckUsR0FBRyxFQUFFO1FBQ0gsTUFBTSxFQUFFLFdBQVc7S0FDcEI7Q0FDRixDQUFDLENBQUM7QUFFSCxHQUFHLENBQUMsS0FBSyxFQUFFLENBQUMiLCJzb3VyY2VzQ29udGVudCI6WyIjIS91c3IvYmluL2VudiBub2RlXG5pbXBvcnQgJ3NvdXJjZS1tYXAtc3VwcG9ydC9yZWdpc3Rlcic7XG5pbXBvcnQgKiBhcyBjZGsgZnJvbSAnYXdzLWNkay1saWInO1xuaW1wb3J0IHsgUnVzdExhbWJkYVN0YWNrIH0gZnJvbSAnLi4vbGliL3J1c3QtbGFtYmRhLXN0YWNrJztcblxuY29uc3QgYXBwID0gbmV3IGNkay5BcHAoKTtcblxubmV3IFJ1c3RMYW1iZGFTdGFjayhhcHAsICdHb2RkYXJkUHJvZFN0YWNrJywge1xuICBkZXNjcmlwdGlvbjogJ0dvZGRhcmQgUHJvZHVjdGlvbiAtIFJ1c3QgTGFtYmRhIEFQSSBkZXBsb3llZCB3aXRoIENESycsXG4gIGVudjoge1xuICAgIHJlZ2lvbjogJ3VzLXdlc3QtMScsXG4gIH0sXG59KTtcblxuYXBwLnN5bnRoKCk7Il19