
use std::collections::HashMap;
use std::convert::TryInto;

use rocket::response::{Flash, Redirect, Responder };
use rocket::request::{ Request, FlashMessage};

use rocket::http::Status;
use rocket::http::{Cookie, CookieJar};

use rocket::State;
use rocket::form::{
    Form,
    Contextual,
    FromForm,
    Context,
};
use rocket_dyn_templates::Template;

use services::printnanny_api::{ ApiService, ServiceError };
use printnanny_api_client::models;

use super::config::{ Config };
use super::response::{ FlashResponse, Response };

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

async fn handle_step1(form: EmailForm<'_>, config: &Config) -> Result<Template, FlashError> {
    let service = ApiService::new(&config.path, &config.base_url, None)?;
    let res = service.auth_email_create(form.email.to_string()).await;
    match res {
        Ok(_) => {
            let redirect = Redirect::to(format!("/login/{}", signup.email));
            Ok((
                Response::Redirect(redirect), Status::new(303)
            ))

        },
        Err(e) => {
            error!("{}",e);
            let mut context = HashMap::new();
            context.insert("errors", format!("Something went wrong {:?}", e));
            Ok((
                Response::Template(Template::render("error", context)),
                Status::new(500),
            ))
        }
    }
}
// NOTE: We use `Contextual` here because we want to collect all submitted form
// fields to re-render forms with submitted values on error. If you have no such
// need, do not use `Contextual`. Use the equivalent of `Form<Submit<'_>>`.
#[post("/", data = "<form>")]
async fn login_step1_submit<'r>(form: Form<Contextual<'r, EmailForm<'r>>>, config: &State<Config>) ->  Result<Template, FlashError> {
    info!("Received auth email form response {:?}", form);
    match form.value {
        Some(signup) => {
            handle_step1(form: signup, config: &config)
        },
        None => {
            info!("form.value is empty");
            Ok((form.context.status(), Response::Template(Template::render("authemail", &form.context))))
        },
    }
}

// NOTE: We use `Contextual` here because we want to collect all submitted form
// fields to re-render forms with submitted values on error. If you have no such
// need, do not use `Contextual`. Use the equivalent of `Form<Submit<'_>>`.

async fn handle_token_validate(token: &str, email: &str, config_path: &str, base_url: &str) -> Result< models::PrintNannyApiConfig, ServiceError>{
    let service = ApiService::new(&config_path, &base_url, None)?;
    let res = service.auth_token_validate(&email, token).await?;
    let bearer_access_token = res.token.to_string();
    let service = ApiService::new(&config_path, base_url, Some(bearer_access_token.clone()))?;
    service.license_download().await?;
    let service = ApiService::new(&config_path, base_url, Some(bearer_access_token.clone()))?;
    let api_config = service.to_api_config()?;
    Ok(api_config)
}

#[post("/<email>", data = "<form>")]
async fn login_step2_submit<'r>(
    email: String,
    jar: &CookieJar<'_>,
    form: Form<Contextual<'r, TokenForm<'r>>>,
    config: &State<Config>) -> Result<FlashResponse<Redirect>, FlashResponse<Redirect>> {
    info!("Received auth email form response {:?}", form);
    match form.value {
        Some(ref v) => {
            let token = v.token;
            let api_config = handle_token_validate(token, &email, &config.path, &config.base_url).await?;
            jar.add_private(Cookie::new("printnanny_api_config", serde_json::to_string(&api_config)));
            Ok(FlashRedirect::from(Flash::success(Redirect::to("/login/success"), "Verification Success")))
        },
        None => {
            info!("form.value is empty");
            Err(FlashRedirect::from(Flash::error(Redirect::to(format!("/login/{}", &email)), "Please enter verification code")))
        },
    }
}


#[get("/<email>")]
fn login_step2(email: String) -> Template {
    let mut context = HashMap::new();
    context.insert("email", email);
    Template::render("authtoken", context)
}

#[get("/")]
fn login_step1() -> Template {
    Template::render("authemail", &Context::default())
}

#[get("/success")]
fn success() -> Template {
    Template::render("success", &Context::default())
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        login_step1,
        login_step1_submit,
        login_step2,
        login_step2_submit,
        success
    ]
}