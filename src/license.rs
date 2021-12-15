use anyhow::{ Result,  anyhow };
use async_trait::async_trait;
use log:: { error, info };

use printnanny_api_client::apis::licenses_api::{
    license_activate,
    licenses_retrieve
};
use printnanny_api_client::models::{ 
    Device,
    TaskStatusType,
    TaskType,
    License,
};

use crate::service::{ PrintNannyService };
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
    let service = PrintNannyService::new(base_dir)?;
    let device = service.get_device().await?;


    check_task_type(&device, TaskType::ActivateLicense)?;
    let last_task = device.last_task.as_ref().unwrap();
    
    service.update_task_status(
        &last_task,
        Some(TaskStatusType::Started),
        Some(msgs::LICENSE_ACTIVATE_STARTED_MSG.to_string()),
        None
    ).await?;
    let activate_result = service.activate_license().await;
    
    match activate_result {
        Ok(result) => {
            let msg = msgs::LICENSE_ACTIVATE_SUCCESS_MSG.to_string();
            info!("{} {:?}", msg, result);
            service.update_task_status(
                &last_task,
                Some(TaskStatusType::Success),
                Some(msg),
                Some(msgs::LICENSE_ACTIVATE_SUCCESS_HELP.to_string())
            ).await?;
            service.device_info_update_or_create().await?;
            service.save().await?;
        },
        Err(e) => {
            let msg = format!("{} {:?}", msgs::LICENSE_ACTIVATE_FAILED_MSG.to_string(), e);
            error!("{}", msg);
            service.update_task_status(
                &last_task,
                Some(TaskStatusType::Failed),
                Some(msg),
                Some(msgs::LICENSE_ACTIVATE_FAILED_HELP.to_string())
            ).await?; 
        }
    }
    Ok(())
}


