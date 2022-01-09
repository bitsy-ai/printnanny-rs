
use std::collections::HashMap;
use std::convert::TryInto;

use rocket::response::{Flash, Redirect};
use rocket::request::FlashMessage;

use rocket::http::Status;
use rocket::http::{Cookie, CookieJar};

use rocket::State;
use rocket::form::{
    Form,
    Contextual,
    FromForm,
    FromFormField,
    Context,
};
use rocket_dyn_templates::Template;

use services::printnanny_api::ApiService;

use super::{ Config, Response };


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


// NOTE: We use `Contextual` here because we want to collect all submitted form
// fields to re-render forms with submitted values on error. If you have no such
// need, do not use `Contextual`. Use the equivalent of `Form<Submit<'_>>`.
#[post("/", data = "<form>")]
async fn login_step1_submit<'r>(form: Form<Contextual<'r, EmailForm<'r>>>, config: &State<Config>) -> (Status, Response) {
    info!("Received auth email form response {:?}", form);
    match form.value {
        Some(ref signup) => {
            let service = ApiService::new(&config.path, &config.base_url).await;
            match service {
                Ok(s) => {
                    let res = s.auth_email_create(signup.email.to_string()).await;
                    match res {
                        Ok(_) => {
                            let redirect = Redirect::to(format!("/login/{}", signup.email));
                            (Status::new(303), Response::Redirect(redirect))

                        },
                        Err(e) => {
                            error!("{}",e);
                            let mut context = HashMap::new();
                            context.insert("errors", format!("Something went wrong {:?}", e));
                            (Status::new(500),  Response::Template(Template::render("error", context)))
                        }
                    }
                },
                Err(e) => {
                    error!("{}",e);
                    let mut context = HashMap::new();
                    context.insert("errors", format!("Something went wrong {:?}", e));
                    (Status::new(500), Response::Template(Template::render("error", &form.context)))
                }
            }
        },
        None => {
            info!("form.value is empty");
            (form.context.status(), Response::Template(Template::render("authemail", &form.context)))
        },
    }
}

// NOTE: We use `Contextual` here because we want to collect all submitted form
// fields to re-render forms with submitted values on error. If you have no such
// need, do not use `Contextual`. Use the equivalent of `Form<Submit<'_>>`.
#[post("/<email>", data = "<form>")]
async fn login_step2_submit<'r>(email: String, jar: &CookieJar<'_>, form: Form<Contextual<'r, TokenForm<'r>>>, config: &State<Config>) -> Result<Flash<Redirect>, Flash<Redirect>> {
    info!("Received auth email form response {:?}", form);
    match form.value {
        Some(ref v) => {
            let service = ApiService::new(&config.path, &config.base_url).await;
            match service {
                Ok(s) => {
                    let token = &v.token;
                    let res = s.auth_token_validate(&email, token).await;
                    match res {
                        Ok(token) => {
                            jar.add_private(Cookie::new("token", token.token));
                            Ok(Flash::success(Redirect::to("/login/success"), "Verification failed."))
                        },
                        Err(e) => {
                            error!("{}",e);
                            Err(Flash::error(Redirect::to(format!("/login/{}", &email)), "Verification failed."))

                        }
                    }
                },
                Err(e) => {
                    error!("{}",e);
                    // let mut context = HashMap::new();
                    // context.insert("errors", format!("Something went wrong {:?}", e));
                    Err(Flash::error(Redirect::to(format!("/login/{}", &email)), "Verification failed."))
                }
            }

        },
        None => {
            info!("form.value is empty");
            Err(Flash::error(Redirect::to(format!("/login/{}", &email)), "Please enter verification code"))
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