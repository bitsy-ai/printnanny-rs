use std::path::{ PathBuf };
use std::fs::{ OpenOptions };

use anyhow::{ Result };
use serde::{ Serialize, Deserialize };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintNannyPath {
    pub backups: PathBuf,
    pub base: PathBuf,
    pub data: PathBuf,
    pub ca_certs: PathBuf,

    // this struct
    pub paths_json: PathBuf,

    // api config
    pub api_config_json: PathBuf,

    // serialized representation contains the "kitchen sink" view of this device
    // with mutable fields and hierarchal/relationship fields serialized (but no guarantee of freshness in local cache)
    // https://github.com/bitsy-ai/printnanny-webapp/blob/55ead2ac638e243a8a5fe6bc046a0120eccd2c78/print_nanny_webapp/devices/api/serializers.py#L124
    pub device_json: PathBuf,
    // contains secrets, tokens deserialized from printnanny_license.zip
    pub license_json: PathBuf,
    pub license_zip: PathBuf,
    // immutable view of device, mostly derived from /proc/cpuinfo
    // this file is created after successful license verification, is used to indicate success of license verification in Ansible task set
    // created by: https://github.com/bitsy-ai/printnanny-webapp/blob/55ead2ac638e243a8a5fe6bc046a0120eccd2c78/print_nanny_webapp/devices/api/serializers.py#L152
    // consumed by: https://github.com/bitsy-ai/ansible-collection-printnanny/blob/9e2ba05526249901a0e29f66a4dce4fd46395045/roles/license/tasks/main.yml#L15

    pub device_info_json: PathBuf,
    pub user_json: PathBuf,
    pub private_key: PathBuf,
    pub public_key: PathBuf,
    pub ca_cert: PathBuf,
    pub ca_cert_backup: PathBuf,
}

impl PrintNannyPath {
    pub fn new(base_str: &str) -> Self {
        let base = PathBuf::from(base_str);
 
        let backups = base.join("backups");
        let data = base.join("data");
        let ca_certs = base.join("ca-certificates");
        let ca_cert= ca_certs.join("gtsltsr.crt");
        let ca_cert_backup = ca_certs.join("GSR4.crt");

        let device_info_json = data.join("device_info.json");
        let api_config_json = data.join("api_config.json");
        let paths_json = data.join("paths.json");

        let user_json = data.join("user.json");
        let device_json = data.join("device.json");
        let license_json = data.join("license.json");
        let license_zip = data.join("license.zip");
        let private_key = data.join("ecdsa256_pkcs8.pem");
        let public_key = data.join("ecdsa_public.pem");

        Self { 
            api_config_json,
            backups,
            base,
            ca_cert_backup,
            ca_cert,
            ca_certs,
            data,
            device_info_json,
            device_json,
            license_json,
            license_zip,
            paths_json,
            private_key,
            public_key,
            user_json,
        }
    }
}

impl PrintNannyPath {
    pub fn save(&self) -> Result<()>{
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.paths_json)?;
        serde_json::to_writer(&file, &self)?;
        Ok(())
    }
}