
use std::collections::HashMap;
use rocket::serde::{ Serialize, Deserialize };
use rocket::response::{Flash, Redirect};

use rocket::http::{Cookie, CookieJar};

use rocket::State;
use rocket::form::{
    Form,
    Contextual,
    FromForm,
    Context,
};
use rocket_dyn_templates::Template;

use printnanny_services::printnanny_api::{ ApiConfig, ApiService, ServiceError };
use printnanny_api_client::models;

use super::config::{ Config };
use super::error;
use super::response::{ FlashResponse, Response };

pub const COOKIE_API_CONFIG: &str = "printnanny_api_config";
// generic
#[derive(Debug, Serialize, Deserialize)]
pub struct DashContext {
    // api_config: models::PrintNannyApiConfig,
    user: models::User,
    device: models::Device,
    // system_info: models::SystemInfo
}

#[derive(Debug, FromForm)]
pub struct EmailForm<'v> {
    #[field(validate = contains('@').or_else(msg!("invalid email address")))]
    email: &'v str,
    #[field(validate = eq(true).or_else(msg!("Please agree to submit anonymous debug logs")))]
    analytics: bool,
}

#[derive(Debug, FromForm)]
pub struct TokenForm<'v> {
    token: &'v str,
}

async fn handle_step1(form: &EmailForm<'_>, config: &Config) -> Result<Response, FlashResponse<Template>> {
    let api_config = ApiConfig{base_path: config.base_url.to_string(), bearer_access_token: None};
    let service = ApiService::new(api_config, &config.path)?;
    let res = service.auth_email_create(form.email.to_string()).await;
    match res {
        Ok(_) => {
            let redirect = Redirect::to(format!("/login/{}", form.email));
            Ok(Response::Redirect(redirect))

        },
        Err(e) => {
            error!("{}",e);
            let mut context = HashMap::new();
            context.insert("errors", format!("Something went wrong {:?}", e));
            Ok(Response::Template(Template::render("error", context)))
        }
    }
}
// NOTE: We use `Contextual` here because we want to collect all submitted form
// fields to re-render forms with submitted values on error. If you have no such
// need, do not use `Contextual`. Use the equivalent of `Form<Submit<'_>>`.
#[post("/", data = "<form>")]
async fn login_step1_submit<'r>(form: Form<Contextual<'r, EmailForm<'r>>>, config: &State<Config>) ->  Result<Response, FlashResponse<Template>> {
    info!("Received auth email form response {:?}", form);
    match &form.value {
        Some(signup) => {
            let result = handle_step1(signup, config).await?;
            Ok(result)
        },
        None => {
            info!("form.value is empty");
            Ok(Response::Template(Template::render("authemail", &form.context)))
        },
    }
}

// NOTE: We use `Contextual` here because we want to collect all submitted form
// fields to re-render forms with submitted values on error. If you have no such
// need, do not use `Contextual`. Use the equivalent of `Form<Submit<'_>>`.

async fn handle_token_validate(token: &str, email: &str, config_path: &str, base_url: &str) -> Result<ApiConfig, ServiceError>{
    let api_config = ApiConfig{base_path: base_url.to_string(), bearer_access_token: None};
    
    let service = ApiService::new(api_config, config_path)?;
    let res = service.auth_token_validate(email, token).await?;
    let bearer_access_token = res.token.to_string();

    let api_config = ApiConfig{base_path: base_url.to_string(), bearer_access_token: Some(bearer_access_token)};
    let service = ApiService::new(api_config.clone(), config_path)?;
    service.device_setup().await?;
    Ok(api_config)
}

#[post("/<email>", data = "<form>")]
async fn login_step2_submit<'r>(
    email: String,
    jar: &CookieJar<'_>,
    form: Form<Contextual<'r, TokenForm<'r>>>,
    config: &State<Config>) -> Result<FlashResponse<Redirect>, FlashResponse<Template>> {
    info!("Received auth email form response {:?}", form);
    match form.value {
        Some(ref v) => {
            let token = v.token;
            let api_config = handle_token_validate(token, &email, &config.path, &config.base_url).await?;
            let cookie_value = serde_json::to_string(&api_config)?;
            jar.add_private(Cookie::new(COOKIE_API_CONFIG, cookie_value));
            Ok(FlashResponse::<Redirect>::from(Flash::success(Redirect::to("/onboarding"), "Verification Success")))
        },
        None => {
            info!("form.value is empty");
            Err(FlashResponse::<Template>::from(error::Error::VerificationFailed{}))
        },
    }
}

#[get("/onboarding")]
fn login_step3(jar: &CookieJar<'_>, config: &State<Config>) ->  Result<Response, FlashResponse<Redirect>>{
    let get_api_config = jar.get_private(COOKIE_API_CONFIG);
    match get_api_config {
        Some(cookie) => {
            let api_config: ApiConfig = serde_json::from_str(cookie.value())?;
            let service = ApiService::new(api_config, &config.path);
            Ok(Response::Template(Template::render("onoarding", &Context::default())))
        },
        None => Ok(Response::Template(Template::render("authemail", &Context::default())))
    }
}

#[get("/<email>")]
fn login_step2(email: String) -> Template {
    let mut context = HashMap::new();
    context.insert("email", email);
    Template::render("authtoken", context)
}

pub async fn get_context(config_path: &str, api_config: &ApiConfig) -> Result<DashContext, ServiceError> {
    let mut service = ApiService::new(api_config.clone(), config_path)?;
    // user into context
    let context = DashContext{
        user: service.user.unwrap(),
        device: service.device.unwrap(),
    };

    Ok(context)
}

#[get("/")]
async fn login_step1(jar: &CookieJar<'_>) -> Result<Response, FlashResponse<Redirect>> {
    let get_api_config = jar.get_private(COOKIE_API_CONFIG);
    match get_api_config {
        Some(_) => {
            Ok(Response::Redirect(Redirect::to("/")))
        },
        None => Ok(Response::Template(Template::render("authemail", &Context::default())))
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        login_step1,
        login_step1_submit,
        login_step2,
        login_step2_submit,
    ]
}