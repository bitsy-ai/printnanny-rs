use rocket::response::{Flash, Redirect};
use rocket::serde::{Deserialize, Serialize};
use std::collections::HashMap;

use rocket::http::{Cookie, CookieJar};

use rocket::form::{Context, Contextual, Form, FromForm};
use rocket::State;
use rocket_dyn_templates::Template;

use printnanny_api_client::models;
use printnanny_services::config::{ApiConfig, PrintNannyConfig};
use printnanny_services::printnanny_api::{ApiService, ServiceError};

use super::error;
use super::response::{FlashResponse, Response};

pub const COOKIE_CONFIG: &str = "printnanny_config";
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

async fn handle_step1(
    form: &EmailForm<'_>,
    config: &State<PrintNannyConfig>,
) -> Result<Response, FlashResponse<Template>> {
    let c = config.inner().clone();
    let service = ApiService::new(c)?;
    let res = service.auth_email_create(form.email.to_string()).await;
    match res {
        Ok(_) => {
            let redirect = Redirect::to(format!("/login/{}", form.email));
            Ok(Response::Redirect(redirect))
        }
        Err(e) => {
            error!("{}", e);
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
async fn login_step1_submit<'r>(
    form: Form<Contextual<'r, EmailForm<'r>>>,
    config: &State<PrintNannyConfig>,
) -> Result<Response, FlashResponse<Template>> {
    info!("Received auth email form response {:?}", form);
    let c = config.inner().clone();

    match &form.value {
        Some(signup) => {
            let result = handle_step1(signup, config).await?;
            Ok(result)
        }
        None => {
            info!("form.value is empty");
            Ok(Response::Template(Template::render(
                "authemail",
                &form.context,
            )))
        }
    }
}

// NOTE: We use `Contextual` here because we want to collect all submitted form
// fields to re-render forms with submitted values on error. If you have no such
// need, do not use `Contextual`. Use the equivalent of `Form<Submit<'_>>`.

async fn handle_token_validate(
    token: &str,
    email: &str,
    config: PrintNannyConfig,
) -> Result<PrintNannyConfig, ServiceError> {
    let mut auth_config = config.clone();
    let service = ApiService::new(config)?;
    let res = service.auth_token_validate(email, token).await?;
    let bearer_access_token = res.token.to_string();
    info!("Success! Authenticated and received bearer token");

    let api_config = ApiConfig {
        base_path: service.config.api.base_path,
        bearer_access_token: Some(bearer_access_token),
    };
    auth_config.api = api_config;
    let service = ApiService::new(auth_config)?;
    info!("Setting up device");
    let device = service.device_setup().await?;
    info!("Success! Device updated: {:?}", device);
    Ok(service.config)
}

#[post("/<email>", data = "<form>")]
async fn login_step2_submit<'r>(
    email: String,
    jar: &CookieJar<'_>,
    form: Form<Contextual<'r, TokenForm<'r>>>,
    config: &State<PrintNannyConfig>,
) -> Result<FlashResponse<Redirect>, FlashResponse<Template>> {
    info!("Received auth email form response {:?}", form);
    let c = config.inner().clone();
    match form.value {
        Some(ref v) => {
            let token = v.token;
            let api_config: PrintNannyConfig = handle_token_validate(token, &email, c).await?;
            let cookie_value = serde_json::to_string(&api_config)?;
            jar.add_private(Cookie::new(COOKIE_CONFIG, cookie_value));
            Ok(FlashResponse::<Redirect>::from(Flash::success(
                Redirect::to("/login/welcome"),
                "Verification Success",
            )))
        }
        None => {
            info!("form.value is empty");
            Err(FlashResponse::<Template>::from(
                error::Error::VerificationFailed {},
            ))
        }
    }
}

#[get("/welcome")]
async fn login_step3(jar: &CookieJar<'_>) -> Result<Response, FlashResponse<Template>> {
    let get_api_config = jar.get_private(COOKIE_CONFIG);
    match get_api_config {
        Some(cookie) => {
            let config: PrintNannyConfig = serde_json::from_str(cookie.value())?;
            let context = get_context(config).await?;
            Ok(Response::Template(Template::render("welcome", context)))
        }
        None => Ok(Response::Template(Template::render(
            "authemail",
            &Context::default(),
        ))),
    }
}

#[get("/<email>")]
fn login_step2(email: String) -> Template {
    let mut context = HashMap::new();
    context.insert("email", email);
    Template::render("authtoken", context)
}

pub async fn get_context(config: PrintNannyConfig) -> Result<DashContext, ServiceError> {
    let service = ApiService::new(config.clone())?;
    let device = service.device_retrieve_hostname().await?;
    let user = service.auth_user_retreive().await?;
    let context = DashContext { user, device };

    Ok(context)
}

#[get("/")]
async fn login_step1(jar: &CookieJar<'_>) -> Result<Response, FlashResponse<Redirect>> {
    let get_api_config = jar.get_private(COOKIE_CONFIG);
    match get_api_config {
        Some(_) => Ok(Response::Redirect(Redirect::to("/"))),
        None => Ok(Response::Template(Template::render(
            "authemail",
            &Context::default(),
        ))),
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        login_step1,
        login_step1_submit,
        login_step2,
        login_step2_submit,
        login_step3
    ]
}
