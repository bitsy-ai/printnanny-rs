use anyhow::{ Result };
use async_trait::async_trait;
use super::generic::{ ApiModel, PrintNannyService };

#[async_trait]
impl ApiModel<Device> for PrintNannyService<Device> {
    async fn retrieve(&self, id: i32) -> Result<Device>{
        Ok(devices_retrieve(&self.request_config, id).await?)
    }
}

impl PrintNannyService<Device> {
}
