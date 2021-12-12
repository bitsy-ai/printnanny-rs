use std::path::{ PathBuf };

#[derive(Debug, Clone)]
pub struct PrintNannyPath {
    pub backups: PathBuf,
    pub base: PathBuf,
    pub data: PathBuf,
    pub ca_certs: PathBuf,
    // serialized representation contains the "kitchen sink" view of this device
    // with mutable fields and hierarchal/relationship fields serialized (but no guarantee of freshness in local cache)
    // https://github.com/bitsy-ai/printnanny-webapp/blob/55ead2ac638e243a8a5fe6bc046a0120eccd2c78/print_nanny_webapp/devices/api/serializers.py#L124
    pub device_json: PathBuf,
    // contains secrets, tokens deserialized from printnanny_license.zip
    pub license_json: PathBuf,
    // immutable view of device, mostly derived from /proc/cpuinfo
    // this file is created after successful license verification, is used to indicate success of license verification in Ansible task set
    // created by: https://github.com/bitsy-ai/printnanny-webapp/blob/55ead2ac638e243a8a5fe6bc046a0120eccd2c78/print_nanny_webapp/devices/api/serializers.py#L152
    // consumed by: https://github.com/bitsy-ai/ansible-collection-printnanny/blob/9e2ba05526249901a0e29f66a4dce4fd46395045/roles/license/tasks/main.yml#L15

    pub device_info_json: PathBuf,
    pub private_key: PathBuf,
    pub public_key: PathBuf,
    pub ca_cert: PathBuf,
    pub ca_cert_backup: PathBuf

}

impl PrintNannyPath {
    pub fn from(base_str: &str) -> Self {
        let base = PathBuf::from(base_str);
 
        let backups = base.join("backups");
        let data = base.join("data");
        let ca_certs = base.join("ca-certificates");
        let ca_cert= ca_certs.join("gtsltsr.crt");
        let ca_cert_backup = ca_certs.join("GSR4.crt");

        let device_info_json = data.join("device_info.json");
        let device_json = data.join("device.json");
        let license_json = data.join("license.json");
        let private_key = data.join("ecdsa256_pkcs8.pem");
        let public_key = data.join("ecdsa_public.pem");

        Self { 
            backups:backups,
            base: base,
            ca_certs: ca_certs,
            ca_cert: ca_cert,
            ca_cert_backup: ca_cert_backup,
            data: data,
            device_info_json: device_info_json,
            license_json: license_json,
            device_json: device_json,
            private_key: private_key,
            public_key: public_key,
        }
    }
}