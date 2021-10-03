
use log::{info };
use anyhow::{ Context, Result };
use thiserror::Error;
use print_nanny_client::models::{ 
    CallbackTokenAuthRequest,
    DetailResponse,
    EmailAuthRequest,
    TokenResponse,
};
use print_nanny_client::apis::auth_api::{ auth_email_create, auth_token_create };
use print_nanny_client::apis::configuration::{ Configuration as PrintNannyAPIConfig };

use crate::config::{ PrintNannySystemConfig };
use crate::device::{ device_identity_update_or_create };
use crate::prompt::{ prompt_device_name, prompt_token_input, prompt_email };

// https://github.com/dtolnay/thiserror
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("🔴 Device not registered. Please run `printnanny auth` to get started")]
    AuthRequired,
}

async fn verify_2fa_send_email(api_config: &PrintNannyAPIConfig, email: &str) -> Result<DetailResponse> {
    // Sends an email containing an expiring one-time password (6 digits)
    let req =  EmailAuthRequest{email:email.to_string()};
    let res = auth_email_create(&api_config, req).await
        .context(format!("🔴 Failed to send verification email to {}", email))?;
    info!("SUCCESS auth_email_create detail {:?}", serde_json::to_string(&res));
    Ok(res)
}

async fn verify_2fa_code(api_config: &PrintNannyAPIConfig, token: String, email: &str) -> Result<TokenResponse> {
    // Verifies email and one-time password (6 digit pair), returning a Bearer token if verification succeeds
    let req = CallbackTokenAuthRequest{mobile: None, token:token, email:Some(email.to_string())};
    let res = auth_token_create(&api_config, req).await
        .context("🔴 Verification failed. Please try again or contact leigh@print-nanny.com for help.")?;
    info!("SUCCESS auth_verify_create detail {:?}", serde_json::to_string(&res));
    Ok(res)
}

pub async fn verify_2fa_auth(api_config: &PrintNannyAPIConfig, email: &str) -> Result<TokenResponse> {
    verify_2fa_send_email(&api_config, email).await?;
    println!("📥 Sent a 6-digit verification code to {}",email);
    let otp_token = prompt_token_input(email);
    println!("✅ Success! Your email was verified {}",email);
    println!("⏳ Registering your device. Please wait for completion.");
    let api_token = verify_2fa_code(&api_config, otp_token, email).await?;
    Ok(api_token)
}

pub async fn auth(config: &mut PrintNannySystemConfig, app_name: &str, config_name: &str) -> Result<()> {
    let email = prompt_email();
    let api_config = print_nanny_client::apis::configuration::Configuration{
        base_path:config.api_url.to_string(), ..Default::default() 
    };
    let token_res = verify_2fa_auth(&api_config, &email).await?;
    config.email = Some(email);
    config.api_token = Some(token_res.token);
    let device_name = prompt_device_name();
    let pki_res = device_identity_update_or_create(config, &device_name).await?;
    println!("✅ Success! Registered device fingerprint {:?}", pki_res.fingerprint);
    config.device_identity = Some(pki_res);
    confy::store(app_name, config_name, config)?;

    Ok(())
}