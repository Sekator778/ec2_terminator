use lambda_runtime::{service_fn, LambdaEvent, Error};
use serde::{Deserialize, Serialize};
use log::{error, info};
use simple_logger::SimpleLogger;
use aws_sdk_ec2::{Client, Error as Ec2Error};

const TAG_NAME: &str = "AutoTerminate";
const TAG_VALUE: &str = "true";

#[derive(Deserialize, Debug)]
struct Request {}

#[derive(Serialize, Debug)]
struct Response {
    msg: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new().init().unwrap();
    let func = service_fn(my_handler);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn my_handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    info!("Received event: {:?}", event);
    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    match stop_instances_with_tag(&client).await {
        Ok(stopped_instances) => {
            let resp = Response {
                msg: format!("Stopped instances: {:?}", stopped_instances),
            };
            info!("Successfully stopped instances: {:?}", stopped_instances);
            Ok(resp)
        }
        Err(e) => {
            error!("Failed to stop instances: {}", e);
            Err(e.into())
        }
    }
}

async fn stop_instances_with_tag(client: &Client) -> Result<Vec<String>, Ec2Error> {
    info!("Fetching instances with tag {}={}", TAG_NAME, TAG_VALUE);
    let instances = client.describe_instances()
        .filters(aws_sdk_ec2::model::Filter::builder()
            .name(&format!("tag:{}", TAG_NAME))
            .values(TAG_VALUE)
            .build())
        .send()
        .await?;

    let mut instance_ids = Vec::new();
    for reservation in instances.reservations().unwrap_or_default() {
        for instance in reservation.instances().unwrap_or_default() {
            if let Some(instance_id) = instance.instance_id() {
                instance_ids.push(instance_id.to_string());
                info!("Found instance with ID: {}", instance_id);
            }
        }
    }

    if !instance_ids.is_empty() {
        info!("Stopping instances with IDs: {:?}", instance_ids);
        client.stop_instances()
            .set_instance_ids(Some(instance_ids.clone()))
            .send()
            .await?;
        info!("Stop request sent for instances: {:?}", instance_ids);
    } else {
        info!("No instances found with tag {}={}", TAG_NAME, TAG_VALUE);
    }

    Ok(instance_ids)
}
