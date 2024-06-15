# EC2 Terminator Lambda Function

## Overview

This project contains a Rust-based AWS Lambda function that terminates EC2 instances based on specific tags. The function is designed to look for EC2 instances with the tag `AutoTerminate` set to `true` and terminate them. It also deletes associated security groups after the instances are terminated.

## Project Structure

- `Cargo.toml`: Contains the metadata and dependencies for the Rust project.
- `Cargo.lock`: Locks the dependencies to specific versions.
- `src/main.rs`: The main Rust source code file containing the Lambda function logic.
- `.gitignore`: Specifies which files and directories Git should ignore.

## Prerequisites

- Rust programming language installed.
- Docker installed (for building the Lambda function for AWS).
- AWS CLI configured with the necessary permissions.

## Constants

- `TAG_NAME`: The name of the tag used to identify EC2 instances for termination.
- `TAG_VALUE`: The value of the tag used to identify EC2 instances for termination.

```rust
const TAG_NAME: &str = "AutoTerminate";
const TAG_VALUE: &str = "true";
```

## Building and Deploying the Lambda Function

### Step 1: Build the Lambda Function

Use Docker to build the Lambda function for the `x86_64-unknown-linux-musl` target:

```sh
docker build -t ec2-terminator .
container_id=$(docker create ec2-terminator)
docker cp ${container_id}:/app/bootstrap ./bootstrap
docker rm ${container_id}
zip lambda.zip bootstrap
```

### Step 2: Deploy the Lambda Function

#### First Deployment

Create the Lambda function using the AWS CLI:

```sh
aws lambda create-function --function-name ec2Terminator \
  --handler bootstrap \
  --runtime provided.al2 \
  --role arn:aws:iam::741238249954:role/service-role/avbo-test-role-h7x0j96b \
  --zip-file fileb://lambda.zip --region eu-central-1
```

#### Redeploying

If the Lambda function already exists, you can update it:

```sh
aws lambda update-function-code --function-name ec2Terminator --zip-file fileb://lambda.zip --region eu-central-1
```

## How It Works

1. **Initialization**:
   - The Lambda function initializes and sets up logging.

2. **Event Handling**:
   - The function is triggered by an AWS event (such as a CloudWatch event or API Gateway request).
   - It retrieves the AWS configuration and creates an EC2 client.

3. **Instance Identification**:
   - The function describes EC2 instances with the tag `AutoTerminate` set to `true`.

4. **Terminating Instances**:
   - It terminates the identified EC2 instances.
   - Waits until the instances are fully terminated.
   - Deletes the associated security groups of the terminated instances.
   - Logs the details of terminated instances and deleted security groups for audit purposes.

5. **Response**:
   - The function returns a response indicating the instances that were terminated and security groups that were deleted.

