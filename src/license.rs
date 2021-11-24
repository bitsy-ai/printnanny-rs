use std::path::{ PathBuf };
use std::fs::{ read_to_string, OpenOptions };
use anyhow::{ Result, Context };
use procfs::{ CpuInfo, Meminfo };
use printnanny_api_client::models::{ Device, DeviceInfoRequest, DeviceInfo };
use printnanny_api_client::apis::configuration::{ Configuration };
use printnanny_api_client::apis::devices_api::{ 
    devices_retrieve, 
    device_info_update_or_create
};
use crate::paths::{ PrintNannyPath };

// The files referenced in this fn are unzipped to correct target location 
// by an Ansible playbook executed in systemd unit on device boot
// ref: https://github.com/bitsy-ai/ansible-collection-printnanny/blob/main/roles/main/tasks/license_install.yml
pub async fn verify_license(base_dir: &str) -> Result<()>{
    let PATHS = PrintNannyPath::from(base_dir);

    // read device config json from disk
    let device = serde_json::from_str::<Device>(
        &read_to_string(PATHS.device_json.clone())
        .context(format!("Failed to read {:?}", PATHS.device_json))?
        )?;
    
    let license = device.active_license.as_ref().unwrap();
    let creds = license.credentials.as_ref().unwrap();
    
    let api_base_path = creds.printnanny_api_url.as_ref().unwrap().to_string();
    let api_token = Some(creds.printnanny_api_token.as_ref().unwrap().to_string());
    let api_config = Configuration{ 
        base_path: api_base_path,
        bearer_access_token: api_token,
        ..Configuration::default()
    };
    verify_remote_device(&api_config, &device).await?;
    let device_info = info_update_or_create(
        &api_config, 
        &device,
        &PATHS.device_info_json
        ).await?;
    println!("Created DeviceInfo {:?}", device_info);
    Ok(())
}

async fn verify_remote_device(api_config: &Configuration, device: &Device) -> Result<()>{
    let device_id = device.id.unwrap();
    let remote_device = devices_retrieve(&api_config, device_id).await
        .context(format!("Failed to retrieve device with id={}", device_id))?;
    assert_eq!(device, &remote_device, "Device verification failed. Please re-download license file to /boot/printnanny_license.zip");
    Ok(())
}

async fn info_update_or_create(api_config: &Configuration, device: &Device, out: &PathBuf) -> Result<DeviceInfo> {
    let machine_id = read_to_string("/etc/machine-id")?;
    let image_version = read_to_string("/boot/image_version.txt")?;
    let cpuinfo = CpuInfo::new()?;
    let unknown = "Unknown".to_string();
    let revision = cpuinfo.fields.get("Revision").unwrap_or(&unknown);
    let hardware = cpuinfo.fields.get("Hardware").unwrap_or(&unknown);
    let model = cpuinfo.fields.get("Model").unwrap_or(&unknown);
    let serial = cpuinfo.fields.get("Serial").unwrap_or(&unknown);
    let cores = cpuinfo.num_cores();
    let meminfo = Meminfo::new()?;
    let ram = meminfo.mem_total;
    let device_id = device.id.unwrap();
    let req = DeviceInfoRequest{
        cores: cores as i32,
        device: device_id,
        hardware: hardware.to_string(),
        machine_id: machine_id,
        model: model.to_string(),
        ram: ram as i64,
        revision: revision.to_string(),
        serial: serial.to_string(),
        image_version: image_version
    };
    let res = device_info_update_or_create(api_config, device_id, req).await?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(out)?;
    serde_json::to_writer(file, &res)
        .context(format!("Failed to save DeviceInfo to {:?}", out))?;
    Ok(res)
}
