mod constants;
mod ec2_manager;

use constants::*;
use ec2_manager::*;
use lambda_runtime::{service_fn, LambdaEvent, Error};
use serde::{Deserialize, Serialize};
use log::{error, info};
use simple_logger::SimpleLogger;
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_lambda::Client as LambdaClient;
use aws_config::meta::region::RegionProviderChain;

#[derive(Deserialize, Debug)]
struct Request {}

#[derive(Serialize, Debug)]
struct Response {
    msg: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new().init().unwrap();
    let region_provider = RegionProviderChain::default_provider().or_else("us-west-2");
    let config = aws_config::load_from_env().await;

    let func = service_fn(my_handler);

    // Adding tags to the Lambda function
    add_lambda_tags(&config).await?;

    lambda_runtime::run(func).await?;
    Ok(())
}

async fn add_lambda_tags(config: &aws_config::SdkConfig) -> Result<(), Error> {
    let client = LambdaClient::new(config);

    client.tag_resource()
        .resource("arn:aws:lambda:eu-central-1:741238249954:function:ec2Terminator")
        .tags(LAMBDA_TAG_COST_GROUP_KEY, LAMBDA_TAG_COST_GROUP_VALUE)
        .tags(LAMBDA_TAG_CUSTOMER_KEY, LAMBDA_TAG_CUSTOMER_VALUE)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to add tags to Lambda function: {}", e);
            e
        })?;

    info!("Successfully added tags to Lambda function");
    Ok(())
}

async fn my_handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    info!("Received event: {:?}", event);
    let region_provider = RegionProviderChain::default_provider().or_else("us-west-2");
    let config = aws_config::load_from_env().await;
    let ec2_client = Ec2Client::new(&config);

    match stop_instances_with_tag(&ec2_client).await {
        Ok(stopped_instances) => {
            delete_attached_volumes(&ec2_client, &stopped_instances).await?;
            delete_security_groups(&ec2_client, &stopped_instances).await?;
            let resp = Response {
                msg: format!("Stopped instances: {:?}, deleted attached volumes, and deleted security groups", stopped_instances),
            };
            info!("Successfully stopped instances, deleted attached volumes, and deleted security groups: {:?}", stopped_instances);
            Ok(resp)
        }
        Err(e) => {
            error!("Failed to stop instances or delete resources: {}", e);
            Err(e.into())
        }
    }
}
