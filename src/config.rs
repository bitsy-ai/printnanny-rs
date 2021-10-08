use log::{ info };
use thiserror::Error;
use anyhow::{ Context, Result };
use dialoguer::{ Input };
use serde::{ Serialize, Deserialize };

use print_nanny_client::apis::auth_api::{ auth_email_create, auth_token_create };
use print_nanny_client::models::{ 
    CallbackTokenAuthRequest,
    DetailResponse,
    EmailAuthRequest,
    TokenResponse,
};
use print_nanny_client::models::{ 
    Appliance,
    User
};

#[derive(Error, Debug)]
pub enum PromptError {
    #[error("🔴 Please enter required field: {0}")]
    Required(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct LocalConfig {
    #[serde(default)]
    pub api_url: String,
    #[serde(default)]
    pub api_token: Option<String>,
    #[serde(default)]
    pub app_name: String,
    #[serde(default)]
    pub email: String,

    pub appliance: Option<Appliance>,
    pub user: Option<User>
}

impl LocalConfig {

    pub fn api_config(&self) -> print_nanny_client::apis::configuration::Configuration {
        if self.api_token.is_none(){
            print_nanny_client::apis::configuration::Configuration{
                base_path:self.api_url.to_string(), 
                ..Default::default()
            }
        } else {
            print_nanny_client::apis::configuration::Configuration{
                base_path:self.api_url.to_string(),
                bearer_access_token:self.api_token.clone(),
                ..Default::default()
            }
        }
    }
    pub fn print_spacer() {
        let (w, _) = term_size::dimensions().unwrap_or((24,24));
        let spacer = (0..w/2).map(|_| "-").collect::<String>();
        info!("{}", spacer);
    }

    pub fn print(&self) {
        LocalConfig::print_spacer();
        info!("💜 Print Nanny config:");
        info!("💜 {:#?}", self);
        LocalConfig::print_spacer();
    }

    pub fn load(app_name: &str) -> Result<LocalConfig, confy::ConfyError> {
        confy::load(app_name)
    }

    pub fn save(&self) -> Result<(), confy::ConfyError> {
        confy::store(&self.app_name, self)
    }

    pub async fn prompt_2fa(mut self) -> Result<LocalConfig> {
        self.email = LocalConfig::prompt_email();
        LocalConfig::verify_2fa_send_email(&self).await?;
        let otp_token = LocalConfig::prompt_token_input(&self);
        let res: TokenResponse = LocalConfig::verify_2fa_code(&self, otp_token).await?;
        self.api_token = Some(res.token);
        LocalConfig::save(&self)?;
        Ok(self)
    }

    async fn verify_2fa_send_email(&self) -> Result<DetailResponse> {
        let api_config = LocalConfig::api_config(self);
        // Sends an email containing an expiring one-time password (6 digits)
        let req =  EmailAuthRequest{email: self.email.clone()};
        let res = auth_email_create(&api_config, req).await
            .context(format!("🔴 Failed to send verification email to {}", self.email))?;
        info!("SUCCESS auth_email_create detail {:?}", serde_json::to_string(&res));
        Ok(res)
    }
    
    async fn verify_2fa_code(&self, token: String) -> Result<TokenResponse> {
        let api_config = LocalConfig::api_config(self);
        let req = CallbackTokenAuthRequest{mobile: None, token, email:Some(self.email.to_string())};
        let res = auth_token_create(&api_config, req).await
            .context("🔴 Verification failed. Please try again or contact leigh@print-nanny.com for help.")?;
        info!("SUCCESS auth_verify_create detail {:?}", serde_json::to_string(&res));
        Ok(res)
    }

    pub fn prompt_email() -> String {
        LocalConfig::print_spacer();
        let prompt = "⚪ Enter your email address";
        Input::new()
            .with_prompt(prompt)
            .interact_text()
            .unwrap()
    }

    pub fn prompt_token_input(&self) -> String {
        let prompt = format!("⚪ Enter the 6-digit code emailed to {}", self.email);
        let input : String = Input::new()
            .with_prompt(prompt)
            .interact_text()
            .unwrap();
        info!("Received input code {}", input);
        input
    }
}

impl ::std::default::Default for LocalConfig {
    fn default() -> Self { Self { 
        api_url: "https://www.print-nanny.com".to_string(),
        api_token: None,
        app_name: "printnanny".to_string(),
        appliance: None,
        email: "".to_string(),
        user: None
    }}
}
