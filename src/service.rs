
use std::fs::{ read_to_string, OpenOptions };
use anyhow::{ anyhow, Context, Result };
use log::{ info };
use procfs::{ CpuInfo, Meminfo };

use printnanny_api_client::apis::configuration::{ Configuration };
use printnanny_api_client::apis::devices_api::{
    devices_tasks_status_create,
    devices_active_license_retrieve,
    devices_retrieve,
    device_info_update_or_create
};
use printnanny_api_client::apis::licenses_api::{
    license_activate,
};
use printnanny_api_client::models::{ 
    Device, 
    DeviceInfo, 
    DeviceInfoRequest,
    License,
    Task,
    TaskStatus,
    TaskStatusRequest,
    TaskStatusType,
};
use crate::paths::{ PrintNannyPath };


#[derive(Debug, Clone)]
pub struct PrintNannyService {
    pub api_config: Configuration,
    pub device: Device,
    pub license: License,
    pub paths: PrintNannyPath,
}

impl PrintNannyService {

    pub fn new(install_dir: &str) -> Result<PrintNannyService>{
        let paths = PrintNannyPath::from(install_dir);
        // read device json from disk
        let device = serde_json::from_str::<Device>(
            &read_to_string(paths.device_json.clone())
            .context(format!("Failed to read {:?}", paths.device_json))?
            )?;
        let license = serde_json::from_str::<License>(
            &read_to_string(paths.license_json.clone())
            .context(format!("Failed to read {:?}", paths.license_json))?
        )?;

        let api_config = Configuration{ 
            base_path: license.printnanny_api_url.clone(),
            bearer_access_token: Some(license.printnanny_api_token.clone()),
            ..Configuration::default()
        };

        let service = PrintNannyService{api_config, device, license, paths};
        Ok(service)
    }


    pub async fn read_device_json(&self) -> Result<String> {
        let device = devices_retrieve(&self.api_config, self.device.id).await?;

        // test serde_json serialization before truncating file
        let result = serde_json::to_string(&device)?;
        Ok(result)
    }

    pub async fn refresh_device_json(&self) -> Result<String> {
        let device = devices_retrieve(&self.api_config, self.device.id).await?;

        // test serde_json serialization before truncating file
        let result = self.read_device_json().await?;

        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.paths.device_json)?;
        serde_json::to_writer(&file, &device)?;
        Ok(result)
    }

    pub async fn update_task_status(
        &self,
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
            &self.api_config, &device_id, task.id, request).await?;
        info!("Created TaskStatus {:?}", result);
        Ok(result)
    }

    pub async fn check_license(&self) -> Result<License>{
        let device_id = self.device.id;
        let active_license = devices_active_license_retrieve(&self.api_config, device_id).await
            .context(format!("Failed to retrieve device with id={}", device_id))?;
        
        if active_license.fingerprint == self.license.fingerprint {
            Ok(active_license)
        } else {
            return Err(anyhow!("License fingerprint {} did not match Device.active_license for device with id={}", self.license.fingerprint, device_id))
        }
    }

    pub async fn activate_license(&self) -> Result<License> {
        let check = self.check_license().await?;
        let license = license_activate(&self.api_config, check.id, None).await?;
        Ok(license)
    }


    pub async fn refresh_license_json(&self) -> Result<String> {
        let license = self.check_license().await?;

        // test serde_json serialization before truncating file
        let result = serde_json::to_string(&license)?;

        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.paths.license_json)?;
        serde_json::to_writer(&file, &license)?;
        Ok(result)
    }

    pub async fn read_license_json(&self) -> Result<String> {
        let license = self.check_license().await?;
        let result = serde_json::to_string(&license)?;
        Ok(result)
    }

    pub async fn device_info_update_or_create(&self) -> Result<DeviceInfo> {
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
        let device_id = self.device.id;
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
        let res = device_info_update_or_create(&self.api_config, device_id, req).await?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.paths.device_info_json)?;
        serde_json::to_writer(file, &res)
            .context(format!("Failed to save DeviceInfo to {:?}", &self.paths.device_info_json))?;
        info!("Created DeviceInfo {:?}", res);
        Ok(res)
    }    
}