
use log::{ info };
// use sysinfo::{ SystemExt };
use thiserror::Error;
use anyhow::{ Context, Result };
use dialoguer::{ Input, MultiSelect, Select };
// use print_nanny_client::models::{ 
//     PrinterProfileRequest,
//     CameraSourceTypeEnum,
//     CameraTypeEnum
// };

// https://github.com/dtolnay/thiserror
#[derive(Error, Debug)]
pub enum PromptError {
    #[error("🔴 Please enter required field: {0}")]
    Required(String),
}

// TODO use Result<String> instead of String type here
pub fn prompt_email() -> String {
    let prompt = "⚪ Enter your email address";
    Input::new()
        .with_prompt(prompt)
        .interact_text()
        .unwrap()
}

pub fn prompt_token_input(email: &str) -> String {
    let prompt = format!("⚪ Enter the 6-digit code emailed to {}", email.to_string());
    let input : String = Input::new()
        .with_prompt(prompt)
        .interact_text()
        .unwrap();
    info!("Received input code {}", input);
    return input;
}
