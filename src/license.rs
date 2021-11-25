use std::path::{ PathBuf };
use std::fs::{ read_to_string, OpenOptions };
use anyhow::{ Result, Context, anyhow };
use log::{ error, info };
use procfs::{ CpuInfo, Meminfo };
use printnanny_api_client::models::{ 
    Device, 
    DeviceInfo, 
    DeviceInfoRequest, 
    SystemTask, 
    SystemTaskRequest,
    SystemTaskStatus,
    SystemTaskType, 
};
use printnanny_api_client::apis::configuration::{ Configuration };
use printnanny_api_client::apis::devices_api::{ 
    devices_system_tasks_create,
    devices_retrieve, 
    device_info_update_or_create
};
use crate::paths::{ PrintNannyPath };
use crate::msgs;

// The files referenced in this fn are unzipped to correct target location 
// by an Ansible playbook executed in systemd unit on device boot
// ref: https://github.com/bitsy-ai/ansible-collection-printnanny/blob/main/roles/main/tasks/license_install.yml
pub async fn verify_license(base_dir: &str) -> Result<()>{
    let paths = PrintNannyPath::from(base_dir);

    // read device config json from disk
    let device = serde_json::from_str::<Device>(
        &read_to_string(paths.device_json.clone())
        .context(format!("Failed to read {:?}", paths.device_json))?
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
    let device_id = device.id.unwrap();
    create_system_task(
        &api_config, 
        device_id,
        SystemTaskType::VerifyLicense,
        SystemTaskStatus::Started,
        Some(msgs::LICENSE_VERIFY_STARTED_MSG.to_string()),
        None
    ).await?;
    let verify_result = verify_remote_device(&api_config, &device).await;
    
    match verify_result {
        Ok(_) => {
            create_system_task(
                &api_config, 
                device_id,
                SystemTaskType::VerifyLicense,
                SystemTaskStatus::Success,
                Some(msgs::LICENSE_VERIFY_SUCCESS_MSG.to_string()),
                Some(msgs::LICENSE_VERIFY_SUCCESS_HELP.to_string())
            ).await?;
            info_update_or_create(
                &api_config, 
                &device,
                &paths.device_info_json
                ).await?;
        },
        Err(_) => {
            create_system_task(
                &api_config, 
                device_id,
                SystemTaskType::VerifyLicense,
                SystemTaskStatus::Failed,
                Some(msgs::LICENSE_VERIFY_FAILED_MSG.to_string()),
                Some(msgs::LICENSE_VERIFY_FAILED_HELP.to_string())
            ).await?; 
        }
    }
    Ok(())
}


async fn create_system_task(
    api_config: &Configuration, 
    device_id: i32, 
    _type: SystemTaskType, 
    status: SystemTaskStatus,
    msg: Option<String>,
    wiki_url: Option<String>
) -> Result<SystemTask> {
    
    let request = SystemTaskRequest{
        status: Some(status), 
        _type: Some(_type), 
        device: device_id,
        ansible_facts: None,
        msg: msg,
        wiki_url: wiki_url
    };
    let result = devices_system_tasks_create(
        api_config, device_id, request).await?;
    info!("Created SystemTask {:?}", result);
    Ok(result)
}

async fn verify_remote_device(api_config: &Configuration, device: &Device) -> Result<()>{
    let device_id = device.id.unwrap();
    let remote_device = devices_retrieve(&api_config, device_id).await
        .context(format!("Failed to retrieve device with id={}", device_id))?;
    
    if device == &remote_device {
        Ok(())
    } else {
        error!("Device verification failed");
        Err(anyhow!("Device verification failed"))
    }
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
    info!("Created DeviceInfo {:?}", res);
    Ok(res)
}
