use anyhow::{ Result };
use async_trait::async_trait;
use printnanny_api_client::apis::devices_api::{
    devices_cloud_iot_devices_retrieve
};
use printnanny_api_client::models::{ 
    CloudiotDevice
};
use super::generic::{ ApiModel, PrintNannyService };

#[async_trait]
impl ApiModel<CloudiotDevice> for PrintNannyService<CloudiotDevice> {
    async fn retrieve(&self, id: i32) -> Result<CloudiotDevice>{
        Ok(devices_cloud_iot_devices_retrieve(&self.request_config, self.license.device, id).await?)
    }
}

impl PrintNannyService<CloudiotDevice> {
}