use std::path::{ PathBuf };
use std::fs;
use std::io::prelude::*;
use sha2::{Sha256, Digest};
use anyhow::{ Context, Result };
use serde::{ Serialize, Deserialize };
use log::{ debug};

use print_nanny_client::apis::configuration::Configuration;
use print_nanny_client::apis::devices_api::{
    devices_keypairs_create
};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyPair {
    pub public_key_path: PathBuf,
    pub public_key_checksum: String,
    private_key_path: PathBuf,
    private_key_checksum: String,
    pub ca_certs_path: PathBuf,
    pub ca_certs_checksum: String,
    pub backup_ca_certs_path: PathBuf,
    pub backup_ca_certs_checksum: String,
}

impl KeyPair {

    pub fn read_private_key(&self) ->  Result<Vec<u8>> {
        let result = fs::read(&self.private_key_path)
            .context(format!("Failed to read file {:?}", &self.private_key_path))?;
        Ok(result)
    }

    pub fn read_public_key(&self) -> Result<Vec<u8>> {
        let result = fs::read(&self.public_key_path)
            .context(format!("Failed to read file {:?}", &self.public_key_path))?;
        Ok(result)
    }

    fn write_and_verify_checksum(filepath: &PathBuf, content: String, checksum: String) -> Result<()> {
        let mut file_w = fs::File::create(filepath)
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
    pub async fn create(path: PathBuf, api_config: &Configuration, device_id: &i32) -> Result<Self> {
        let res = devices_keypairs_create(&api_config, *device_id).await?;

        let public_key_path = path.join("id_dsa.pub");
        let public_key_checksum = res.public_key_checksum.unwrap();
        KeyPair::write_and_verify_checksum(
            &public_key_path,
            res.public_key.unwrap(),
            public_key_checksum.clone()
        )?;

        let private_key_path = path.join("id_dsa");
        let private_key_checksum = res.private_key_checksum.unwrap();
        KeyPair::write_and_verify_checksum(
            &private_key_path,
            res.private_key.unwrap(),
            private_key_checksum.clone()
        )?;

        let ca_certs = res.ca_certs.unwrap();

        let ca_certs_path = path.join("ca_certs.pem");
        let primary_ca_certs = ca_certs.primary.unwrap();
        let ca_certs_checksum = ca_certs.primary_checksum.unwrap();
        KeyPair::write_and_verify_checksum(
            &ca_certs_path,
            primary_ca_certs,
            ca_certs_checksum.clone()
        )?;

        let backup_ca_certs_path = path.join("ca_certs.pem.bak");
        let backup_ca_certs = ca_certs.backup.unwrap();
        let backup_ca_certs_checksum = ca_certs.backup_checksum.unwrap();
        KeyPair::write_and_verify_checksum(
            &backup_ca_certs_path,
            backup_ca_certs,
            backup_ca_certs_checksum.clone()
        )?;

        Ok(Self {
            public_key_path, 
            public_key_checksum, 
            private_key_path, 
            private_key_checksum,
            ca_certs_path,
            ca_certs_checksum,
            backup_ca_certs_path,
            backup_ca_certs_checksum,
        })
    }
}