use anyhow::{ Result,  anyhow };
use printnanny_api_client::models::{ 
    Device,
    TaskStatusType,
    TaskType,
};

use crate::service::PrintNannyService;
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


    check_task_type(&service.device, TaskType::ActivateLicense)?;
    let last_task = service.device.last_task.as_ref().unwrap();
    
    service.update_task_status(
        &last_task,
        Some(TaskStatusType::Started),
        Some(msgs::LICENSE_ACTIVATE_STARTED_MSG.to_string()),
        None
    ).await?;
    let activate_result = service.activate_device().await;
    
    match activate_result {
        Ok(_) => {
            service.update_task_status(
                &last_task,
                Some(TaskStatusType::Success),
                Some(msgs::LICENSE_ACTIVATE_SUCCESS_MSG.to_string()),
                Some(msgs::LICENSE_ACTIVATE_SUCCESS_HELP.to_string())
            ).await?;
            service.device_info_update_or_create().await?;
        },
        Err(_) => {
            service.update_task_status(
                &last_task,
                Some(TaskStatusType::Failed),
                Some(msgs::LICENSE_ACTIVATE_FAILED_MSG.to_string()),
                Some(msgs::LICENSE_ACTIVATE_FAILED_HELP.to_string())
            ).await?; 
        }
    }
    Ok(())
}
