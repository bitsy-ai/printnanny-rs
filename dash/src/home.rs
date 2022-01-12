use rocket::http::{CookieJar};
use rocket_dyn_templates::Template;
use rocket::form::Context;
use rocket::State;
use rocket::response::Redirect;

use super::auth;
use super::config::{ Config };
use super::response::{ Response, FlashResponse };

#[get("/")]
async fn index(jar: &CookieJar<'_>, config: &State<Config>) -> Result<Response, FlashResponse<Redirect>> {
    let api_config = jar.get_private(&auth::CookieLookup::PrintNannyApiConfig.to_string());
    match api_config {
        Some(cookie) => {
            let api_config = serde_json::from_str(cookie.value())?;
            let context = auth::get_context(&config.path, &api_config).await?;
            Ok(Response::Template(Template::render("index", context)))
        },
        None => Ok(Response::Template(Template::render("authemail", &Context::default())))
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        index
    ]
}