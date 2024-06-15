use aws_sdk_ec2::{Client as Ec2Client, Error as Ec2Error, types::Filter};
use log::info;

pub const TAG_NAME: &str = "AutoTerminate";
pub const TAG_VALUE: &str = "true";

pub async fn stop_instances_with_tag(client: &Ec2Client) -> Result<Vec<String>, Ec2Error> {
    info!("Fetching instances with tag {}={}", TAG_NAME, TAG_VALUE);
    let instances = client.describe_instances()
        .filters(Filter::builder()
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

pub async fn delete_attached_volumes(client: &Ec2Client, instance_ids: &[String]) -> Result<(), Ec2Error> {
    for instance_id in instance_ids {
        let volumes = client.describe_volumes()
            .filters(Filter::builder()
                .name("attachment.instance-id")
                .values(instance_id)
                .build())
            .send()
            .await?;

        for volume in volumes.volumes().unwrap_or_default() {
            if let Some(volume_id) = volume.volume_id() {
                info!("Deleting volume with ID: {}", volume_id);
                client.delete_volume()
                    .volume_id(volume_id)
                    .send()
                    .await?;
                info!("Deleted volume with ID: {}", volume_id);
            }
        }
    }
    Ok(())
}

pub async fn delete_security_groups(client: &Ec2Client, instance_ids: &[String]) -> Result<(), Ec2Error> {
    for instance_id in instance_ids {
        let instances = client.describe_instances()
            .instance_ids(instance_id)
            .send()
            .await?;

        for reservation in instances.reservations().unwrap_or_default() {
            for instance in reservation.instances().unwrap_or_default() {
                if let Some(sgs) = instance.security_groups() {
                    for sg in sgs {
                        if let Some(sg_id) = sg.group_id() {
                            info!("Deleting security group with ID: {}", sg_id);
                            client.delete_security_group()
                                .group_id(sg_id)
                                .send()
                                .await?;
                            info!("Deleted security group with ID: {}", sg_id);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_ec2::config::Config;
    use tokio_test::block_on;

    #[tokio::test]
    async fn test_stop_instances_with_tag() {
        let config = Config::builder().build();
        let client = Ec2Client::from_conf(config);
        let result = stop_instances_with_tag(&client).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_attached_volumes() {
        let config = Config::builder().build();
        let client = Ec2Client::from_conf(config);
        let instance_ids = vec!["i-1234567890abcdef0".to_string()];
        let result = delete_attached_volumes(&client, &instance_ids).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_security_groups() {
        let config = Config::builder().build();
        let client = Ec2Client::from_conf(config);
        let instance_ids = vec!["i-1234567890abcdef0".to_string()];
        let result = delete_security_groups(&client, &instance_ids).await;
        assert!(result.is_ok());
    }
}
