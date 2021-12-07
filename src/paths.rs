use std::path::{ PathBuf };

#[derive(Debug, Clone)]
pub struct PrintNannyPath {
    pub facts_path: PathBuf,
    pub device_facts: PathBuf,
    pub license_facts: PathBuf,
    pub base: PathBuf,
    pub data: PathBuf,
    pub backups: PathBuf,
    pub private_key: PathBuf,
    pub public_key: PathBuf,
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
    pub device_info_json: PathBuf
}

impl PrintNannyPath {
    pub fn from(base_str: &str) -> Self {
        let base = PathBuf::from(base_str);
 
        let backups = base.join("backups");
        let data = base.join("data");
        let device_info_json = data.join("device_info.json");
        let device_json = data.join("device.json");
        let facts_path = base.join("facts.d");
        let license_json = data.join("license.json");
        let private_key = data.join("ecdsa256.pem");
        let public_key = data.join("ecdsa_public.pem");

        let device_facts = facts_path.join("device.fact");
        let license_facts = facts_path.join("license.fact");
        Self { 
            backups:backups,
            base: base,
            data: data,
            device_info_json: device_info_json,
            device_json: device_json,
            facts_path: facts_path,
            license_json: license_json,
            private_key: private_key,
            public_key: public_key,
            device_facts: device_facts,
            license_facts: license_facts
        }
    }
}