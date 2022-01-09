
use std::collections::HashMap;
use std::convert::TryInto;

use rocket::response::Redirect;
use rocket::http::Status;

use rocket::State;
use rocket::form::{
    Form,
    Contextual,
    FromForm,
    FromFormField,
    Context,
};
use rocket_auth::{ Users, User };
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
#[post("/login", data = "<form>")]
pub async fn login_step1_submit<'r>(form: Form<Contextual<'r, EmailForm<'r>>>, config: &State<Config>) -> (Status, Response) {
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
#[post("/login/<email>", data = "<form>")]
pub async fn login_step2_submit<'r>(email: String, form: Form<Contextual<'r, TokenForm<'r>>>, config: &State<Config>) -> (Status, Response) {
    info!("Received auth email form response {:?}", form);
    match form.value {
        Some(ref v) => {
            let service = ApiService::new(&config.path, &config.base_url).await;
            match service {
                Ok(s) => {
                    let token = &v.token;
                    let res = s.auth_token_validate(&email, token).await;
                    match res {
                        Ok(_) => {
                            // let redirect = Redirect::to(format!("/success"));
                            (Status::new(200), Response::Template(Template::render("success", &form.context)))

                        },
                        Err(e) => {
                            error!("{}",e);
                            (Status::new(500), Response::Template(Template::render("error", &form.context)))
                        }
                    }
                },
                Err(e) => {
                    error!("{}",e);
                    (Status::new(500), Response::Template(Template::render("error", &form.context)))
                }
            }
        },
        None => {
            info!("form.value is empty");
            (Status::new(500), Response::Template(Template::render("error", &form.context)))
        },
    }
}


#[get("/login/<email>")]
pub fn login_step2(email: String) -> Template {
    let mut context = HashMap::new();
    context.insert("email", email);
    Template::render("authtoken", context)
}

#[get("/login")]
pub fn login_step1(option: Option<User>) -> Template {
    Template::render("authemail", &Context::default())
}