use std::path::{ PathBuf };
use std::fs::{ read_to_string, OpenOptions };
use anyhow::{ Result, Context, anyhow };
use log::{ error, info };
use procfs::{ CpuInfo, Meminfo };
use printnanny_api_client::models::{ 
    Device, 
    DeviceInfo, 
    DeviceInfoRequest,
    License,
    Task,
    TaskRequest,
    TaskStatus,
    TaskStatusRequest,
    TaskStatusType,
    TaskType,
};

use printnanny_api_client::apis::configuration::{ Configuration };
use printnanny_api_client::apis::licenses_api::{
    license_activate
};
use printnanny_api_client::apis::devices_api::{
    devices_tasks_status_create,
    devices_retrieve, 
    device_info_update_or_create
};
use crate::paths::{ PrintNannyPath };
use crate::msgs;

fn check_task_type(device: &Device, expected_type: TaskType) -> Result<()>{
    match &device.last_task {
        Some(last_task) => {
            if last_task.task_type.unwrap() != expected_type {
                return Err(anyhow!("Expected Device.last_task to be {:?} but received task {:?}", expected_type, last_task))
            } else { Ok(()) }
        },
        None => {
            return Err(anyhow!("Expected Device.last_task to be {:?} but received task None", expected_type))
        }
    }
}
// The files referenced in this fn are unzipped to correct target location 
// by an Ansible playbook executed in systemd unit on device boot
// ref: https://github.com/bitsy-ai/ansible-collection-printnanny/blob/main/roles/main/tasks/license_install.yml
pub async fn activate_license(base_dir: &str) -> Result<()>{
    let paths = PrintNannyPath::from(base_dir);

    // read device config json from disk
    let device = serde_json::from_str::<Device>(
        &read_to_string(paths.device_json.clone())
        .context(format!("Failed to read {:?}", paths.device_json))?
        )?;
    
    check_task_type(&device, TaskType::ActivateLicense)?;
    let last_task = device.last_task.as_ref().unwrap();
    let license = device.active_license.as_ref().unwrap();
    
    let api_base_path = license.credentials.printnanny_api_url.as_ref().unwrap().to_string();
    let api_token = Some(license.credentials.printnanny_api_token.as_ref().unwrap().to_string());
    let api_config = Configuration{ 
        base_path: api_base_path,
        bearer_access_token: api_token,
        ..Configuration::default()
    };
    update_task_status(
        &api_config, 
        &last_task,
        Some(TaskStatusType::Started),
        Some(msgs::LICENSE_ACTIVATE_STARTED_MSG.to_string()),
        None
    ).await?;
    let activate_result = activate_remote_device(&api_config, &device, &license).await;
    
    match activate_result {
        Ok(_) => {
            update_task_status(
                &api_config, 
                &last_task,
                Some(TaskStatusType::Success),
                Some(msgs::LICENSE_ACTIVATE_SUCCESS_MSG.to_string()),
                Some(msgs::LICENSE_ACTIVATE_SUCCESS_HELP.to_string())
            ).await?;
            info_update_or_create(
                &api_config, 
                &device,
                &paths.device_info_json
                ).await?;
        },
        Err(_) => {
            update_task_status(
                &api_config, 
                &last_task,
                Some(TaskStatusType::Failed),
                Some(msgs::LICENSE_ACTIVATE_FAILED_MSG.to_string()),
                Some(msgs::LICENSE_ACTIVATE_FAILED_HELP.to_string())
            ).await?; 
        }
    }
    Ok(())
}


async fn update_task_status(
    api_config: &Configuration,
    task: &Task,
    status: Option<TaskStatusType>,
    detail: Option<String>,
    wiki_url: Option<String>,
) -> Result<TaskStatus> {
    
    let request = TaskStatusRequest{
        detail, wiki_url, status, task: task.id
    };
    let device_id = task.device.to_string();
    let result = devices_tasks_status_create(
        api_config, &device_id, task.id, request).await?;
    info!("Created TaskStatus {:?}", result);
    Ok(result)
}

async fn activate_remote_device(api_config: &Configuration, device: &Device, license: &License) -> Result<License>{
    let device_id = device.id;
    let remote_device = devices_retrieve(&api_config, device_id).await
        .context(format!("Failed to retrieve device with id={}", device_id))?;
    
    let result = match remote_device.active_license {
        Some(active_license) => {
            if active_license.fingerprint == license.fingerprint {
                Ok(())
            } else {
                return Err(anyhow!("License fingerprint {} did not match Device.active_license for device with id={}", license.fingerprint, device_id))
            }
        },
        None => {
            return Err(anyhow!("Device with id={} has no active license set", device_id))
        }
    };

    match result {
        Ok(_) => {
            let result = license_activate(&api_config, license.id, None).await?;
            Ok(result)
        },
        Err(e) => e
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
    let device_id = device.id;
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
