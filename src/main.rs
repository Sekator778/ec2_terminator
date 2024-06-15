use lambda_runtime::{service_fn, LambdaEvent, Error};
use serde::{Deserialize, Serialize};
use log::{error, info};
use simple_logger::SimpleLogger;
use aws_sdk_ec2::{Client, Error as Ec2Error};
use std::collections::HashSet;
use std::time::Instant;

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
    let start_time = Instant::now();
    info!("Received event: {:?}", event);
    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    match terminate_instances_with_tag(&client).await {
        Ok(terminated_instances) => {
            let elapsed_time = start_time.elapsed();
            let resp = Response {
                msg: format!("Terminated instances: {:?} in {:?}", terminated_instances, elapsed_time),
            };
            info!("Successfully terminated instances: {:?} in {:?}", terminated_instances, elapsed_time);
            Ok(resp)
        }
        Err(e) => {
            error!("Failed to terminate instances: {}", e);
            Err(e.into())
        }
    }
}

async fn terminate_instances_with_tag(client: &Client) -> Result<Vec<String>, Ec2Error> {
    info!("Fetching instances with tag {}={}", TAG_NAME, TAG_VALUE);
    let instances = client.describe_instances()
        .filters(aws_sdk_ec2::model::Filter::builder()
            .name(&format!("tag:{}", TAG_NAME))
            .values(TAG_VALUE)
            .build())
        .send()
        .await?;

    let mut instance_ids = Vec::new();
    let mut security_group_ids = HashSet::new();

    for reservation in instances.reservations().unwrap_or_default().iter() {
        for instance in reservation.instances().unwrap_or_default().iter() {
            if let Some(instance_id) = instance.instance_id() {
                instance_ids.push(instance_id.to_string());
                info!("Found instance with ID: {}", instance_id);
            }
            for sg in instance.security_groups().unwrap_or_default().iter() {
                if let Some(sg_id) = sg.group_id() {
                    security_group_ids.insert(sg_id.to_string());
                    info!("Found security group with ID: {}", sg_id);
                }
            }
        }
    }

    if !instance_ids.is_empty() {
        info!("Terminating instances with IDs: {:?}", instance_ids);
        client.terminate_instances()
            .set_instance_ids(Some(instance_ids.clone()))
            .send()
            .await?;
        info!("Terminate request sent for instances: {:?}", instance_ids);

        // Wait for instances to be terminated
        let mut terminated = false;
        while !terminated {
            let mut terminated_instance_count = 0;
            let describe_instances_output = client.describe_instances()
                .filters(aws_sdk_ec2::model::Filter::builder()
                    .name("instance-id")
                    .set_values(Some(instance_ids.clone()))
                    .build())
                .send()
                .await?;

            for reservation in describe_instances_output.reservations().unwrap_or_default() {
                for instance in reservation.instances().unwrap_or_default() {
                    if let Some(state) = instance.state() {
                        if state.name() == Some(&aws_sdk_ec2::model::InstanceStateName::Terminated) {
                            terminated_instance_count += 1;
                        }
                    }
                }
            }

            if terminated_instance_count == instance_ids.len() {
                terminated = true;
            } else {
                info!("Waiting for instances to terminate...");
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            }
        }

        // Delete associated security groups
        delete_security_groups(client, &security_group_ids).await?;
    } else {
        info!("No instances found with tag {}={}", TAG_NAME, TAG_VALUE);
    }

    Ok(instance_ids)
}

async fn delete_security_groups(client: &Client, security_group_ids: &HashSet<String>) -> Result<(), Ec2Error> {
    for sg_id in security_group_ids.iter() {
        match client.delete_security_group().group_id(sg_id).send().await {
            Ok(_) => {
                info!("Deleted security group with ID: {}", sg_id);
            }
            Err(e) => {
                error!("Failed to delete security group with ID {}: {}", sg_id, e);
            }
        }
    }
    Ok(())
}
