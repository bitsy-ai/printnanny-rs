use indexmap::indexmap;
use rocket::response::Redirect;
use rocket::serde::{Deserialize, Serialize};
use std::collections::HashMap;

use printnanny_services::config::{ApiConfig, PrintNannyConfig};
use printnanny_services::printnanny_api::{ApiService, ServiceError};
use rocket::form::{Context, Contextual, Form, FromForm};
use rocket::http::{Cookie, CookieJar};
use rocket::State;
use rocket_dyn_templates::Template;

use super::error;
use super::response::{FlashResponse, Response};

pub const COOKIE_CONFIG: &str = "printnanny_config";

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
    let service = ApiService::new(config)?;
    let new_config = service.device_setup().await?;
    info!("Success! Config updated: {:?}", &new_config);
    Ok(new_config)
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

    let api_config = ApiConfig {
        base_path: service.config.api.base_path,
        bearer_access_token: Some(bearer_access_token),
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
    config: &State<PrintNannyConfig>,
) -> Result<Response, FlashResponse<Template>> {
    info!("Received auth email form response {:?}", form);
    let c = config.inner().clone();
    match form.value {
        Some(ref v) => {
            let token = v.token;
            let api_config: PrintNannyConfig = handle_token_validate(token, &email, c).await?;
            let cookie_value = serde_json::to_string(&api_config)?;
            jar.add_private(Cookie::new(COOKIE_CONFIG, cookie_value));
            Ok(Response::Redirect(Redirect::to("/")))
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
            Ok(Response::Template(Template::render("welcome", config)))
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

#[get("/?<email>")]
async fn login_step1_email_prepopulated(
    email: &str,
    jar: &CookieJar<'_>,
) -> Result<Response, FlashResponse<Redirect>> {
    let get_api_config = jar.get_private(COOKIE_CONFIG);
    let mut c = HashMap::new();
    c.insert("values", indexmap! {"email" => vec![email]});
    c.insert("errors", indexmap! {});
    match get_api_config {
        Some(_) => Ok(Response::Redirect(Redirect::to("/"))),
        None => Ok(Response::Template(Template::render("authemail", c))),
    }
}

#[get("/")]
async fn login_step1(jar: &CookieJar<'_>) -> Result<Response, FlashResponse<Redirect>> {
    let get_api_config = jar.get_private(COOKIE_CONFIG);
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
        login_step3
    ]
}
