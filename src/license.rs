use std::path::{ PathBuf };
use std::fs::{ File, read_to_string };
use std::io::{ BufReader };
use anyhow::{ Result, Context };
use serde::{ Serialize, Deserialize };
use serde::de::{ DeserializeOwned };
use print_nanny_client::models::{ ApiConfig, Device };
use print_nanny_client::apis::configuration::{ Configuration };

// The files referenced in this fn are unzipped to correct target location 
// by an Ansible playbook executed in systemd unit on device boot
// ref: https://github.com/bitsy-ai/ansible-collection-printnanny/blob/main/roles/main/tasks/license_install.yml
pub fn verify_license(base_dir: &str) -> Result<()>{
    let base_path = PathBuf::from(base_dir);

    // read api & device config json from disk
    let api_creds = serde_json::from_str::<ApiConfig>(&read_to_string(base_path.join("printnanny_api_config.json"))?)?;
    let device = serde_json::from_str::<Device>(&read_to_string(base_path.join("printnanny_device.json"))?)?;
    // let reader = get_reader(&base_path, "printnanny_device.json")?;
    // let local_device = serde_json::from_reader(reader)?;

    Ok(())
}