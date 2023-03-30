use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct SnapshotClient {
    base_url: String,
}

impl SnapshotClient {
    pub fn build(base_url: String) -> Self {
        Self { base_url }
    }

    pub async fn get_latest_snapshot(&self) -> Result<Bytes, reqwest::Error> {
        let res = reqwest::get(&self.base_url).await?.bytes().await?;
        Ok(res)
    }
}

impl Default for SnapshotClient {
    fn default() -> Self {
        Self {
            base_url: "http://localhost/printnanny-snapshot/jpeg/".to_string(),
        }
    }
}
