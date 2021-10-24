use std::path::{ PathBuf };
use std::fs::File;
use std::io::prelude::*;
use sha2::{Sha256, Digest};
use anyhow::{ Context, Result };
use serde::{ Serialize, Deserialize };
use log::{ info, error, debug, warn };

use print_nanny_client::apis::configuration::Configuration;
use print_nanny_client::apis::appliances_api::{
    appliances_keypairs_create
};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyPair {
    public_key_path: PathBuf,
    public_key_checksum: String,
    private_key_path: PathBuf,
    private_key_checksum: String
}

impl KeyPair {

    fn write_and_verify_checksum(filepath: &PathBuf, content: String, checksum: String) -> Result<()> {
        let mut file_w = File::create(filepath)
            .context(format!("Failed to create file {:#?}", filepath))?;
        file_w.write_all(content.as_bytes())?;
        debug!("Wrote key to {:?}", filepath);
        let contents = std::fs::read_to_string(filepath)?;
        // create a Sha256 object
        let mut hasher = Sha256::new();
        // write input message
        hasher.update(contents);
        // read hash digest and consume hasher
        let buf = hasher.finalize();
        assert_eq!(format!("{:x}", buf), checksum);
        Ok(())
    }
    pub async fn create(path: PathBuf, api_config: &Configuration, appliance_id: &i32) -> Result<Self> {
        let res = appliances_keypairs_create(&api_config, *appliance_id).await?;
        let public_key_path = path.join("id_dsa.pub");
        let public_key_checksum = res.public_key_checksum.unwrap();
        let private_key_path = path.join("id_dsa");
        let private_key_checksum = res.private_key_checksum.unwrap();

        KeyPair::write_and_verify_checksum(
            &private_key_path,
            res.private_key.unwrap(),
            private_key_checksum.clone()
        )?;

        KeyPair::write_and_verify_checksum(
            &public_key_path,
            res.public_key.unwrap(),
            public_key_checksum.clone()
        )?;

        Ok(Self { public_key_path, public_key_checksum, private_key_path, private_key_checksum })
    }
}