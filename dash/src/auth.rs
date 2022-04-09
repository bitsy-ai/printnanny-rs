use std::collections::HashMap;

use indexmap::indexmap;
use rocket::form::{Context, Contextual, Form, FromForm};
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use rocket_dyn_templates::Template;

use printnanny_api_client::models;
use printnanny_services::config::PrintNannyConfig;
use printnanny_services::printnanny_api::{ApiService, ServiceError};

use super::response::Response;

#[derive(Debug, PartialEq, Clone)]
pub struct PrintNannyConfigFile(pub Option<String>);

pub const COOKIE_USER: &str = "printnanny_user";

pub fn is_auth_valid(
    jar: &CookieJar<'_>,
    config_file: &State<PrintNannyConfigFile>,
) -> Result<Option<PrintNannyConfig>, ServiceError> {
    let cookie = jar.get_private(COOKIE_USER);
    match cookie {
        Some(user_json) => {
            let user: models::User = serde_json::from_str(user_json.value())?;
            let config = PrintNannyConfig::new(config_file.0.as_deref())?;

            // if config + cookie mismatch, nuke cookie (profile switch in developer mode)
            if config.user != Some(user.clone()) {
                warn!("config.user {:?} did not match COOKIE_USER {:?}, deleting cookie to force re-auth", config.user, &user);
                jar.remove_private(Cookie::named(COOKIE_USER));
                config.try_factory_reset()?;
                Ok(None)
            } else {
                info!("Auth success! COOKIE_USER matches config.user");
                Ok(Some(config))
            }
        }
        None => Ok(None),
    }
}

#[derive(Debug, FromForm, Serialize, Deserialize)]
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
    config: PrintNannyConfig,
) -> Result<Response, Response> {
    let service = ApiService::new(config)?;
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
    config_file: &State<PrintNannyConfigFile>,
) -> Result<Response, Response> {
    info!("Received auth email form response {:?}", form);
    let config = PrintNannyConfig::new(config_file.0.as_deref())?;
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

pub async fn handle_device_update(
    config: PrintNannyConfig,
) -> Result<PrintNannyConfig, ServiceError> {
    info!("Sending device info using config {:?}", &config);
    let mut service = ApiService::new(config)?;
    let new_config = service.device_setup().await?;
    info!("Success! Config updated: {:?}", &new_config);
    Ok(service.config.clone())
}

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

    let api_config = models::PrintNannyApiConfig {
        base_path: service.config.api.base_path,
        bearer_access_token: Some(bearer_access_token),
        ..auth_config.api
    };
    auth_config.api = api_config;
    let updated_config = handle_device_update(auth_config).await?;
    info!("Success! Config updated: {:?}", updated_config);
    Ok(updated_config)
}

#[post("/<email>", data = "<form>")]
async fn login_step2_submit<'r>(
    email: String,
    jar: &CookieJar<'_>,
    form: Form<Contextual<'r, TokenForm<'r>>>,
    config_file: &State<PrintNannyConfigFile>,
) -> Result<Response, Response> {
    info!("Received auth email form response {:?}", form);
    let config = PrintNannyConfig::new(config_file.0.as_deref())?;
    match form.value {
        Some(ref v) => {
            let token = v.token;
            let api_config: PrintNannyConfig = handle_token_validate(token, &email, config).await?;
            let cookie_value =
                serde_json::to_string(&api_config.user.expect("Failed to read user"))?;
            info!(
                "Saving COOKIE_USER={} value={}",
                &COOKIE_USER, &cookie_value
            );
            jar.add_private(Cookie::new(COOKIE_USER, cookie_value));
            Ok(Response::Redirect(Redirect::to("/")))
        }
        None => {
            info!("form.value is empty");
            Ok(Response::Template(Template::render("authemail", config)))
        }
    }
}

#[get("/<email>")]
fn login_step2(email: String) -> Template {
    let mut context = HashMap::new();
    context.insert("email", email);
    Template::render("authtoken", context)
}

#[get("/?<email>")]
async fn login_step1_email_prepopulated(
    email: &str,
    jar: &CookieJar<'_>,
) -> Result<Response, Response> {
    let get_api_config = jar.get_private(COOKIE_USER);
    let mut c = HashMap::new();
    c.insert("values", indexmap! {"email" => vec![email]});
    c.insert("errors", indexmap! {});
    match get_api_config {
        Some(_) => Ok(Response::Redirect(Redirect::to("/"))),
        None => Ok(Response::Template(Template::render("authemail", c))),
    }
}

#[get("/")]
async fn login_step1(jar: &CookieJar<'_>) -> Result<Response, Response> {
    let get_api_config = jar.get_private(COOKIE_USER);
    match get_api_config {
        Some(_) => Ok(Response::Redirect(Redirect::to("/"))),
        None => Ok(Response::Template(Template::render(
            "authemail",
            Context::default(),
        ))),
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        login_step1,
        login_step1_email_prepopulated,
        login_step1_submit,
        login_step2,
        login_step2_submit,
    ]
}
