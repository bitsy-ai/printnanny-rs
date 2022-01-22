use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintNannyPath {
    pub backups: PathBuf,
    pub base: PathBuf,
    pub data: PathBuf,
    pub keys: PathBuf,
    pub ca_certs: PathBuf,

    // this struct
    pub paths_json: PathBuf,

    // api config
    pub api_config_json: PathBuf,
    pub janus_admin_secret: PathBuf,
    pub janus_token: PathBuf,

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
        let keys = base.join("keys");

        let ca_certs = base.join("ca-certificates");
        let ca_cert = ca_certs.join("gtsltsr.crt");
        let ca_cert_backup = ca_certs.join("GSR4.crt");

        let api_config_json = data.join("api_config.toml");
        let paths_json = data.join("paths.toml");

        let private_key = keys.join("id_ecdsa");
        let public_key = keys.join("id_ecdsa.pub");

        let janus_admin_secret = keys.join("janus_admin_secret");
        let janus_token = keys.join("janus_token");

        Self {
            api_config_json,
            backups,
            base,
            ca_cert_backup,
            ca_cert,
            ca_certs,
            data,
            keys,
            paths_json,
            private_key,
            public_key,
            janus_admin_secret,
            janus_token,
        }
    }
}
