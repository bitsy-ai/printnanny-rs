use log::info;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket_dyn_templates::Template;
use serde_json::{ Value, Map Object };

use super::auth;
use super::response::Response;
use printnanny_services::config::PrintNannyConfig;

pub enum DashContext<'a> {
    Config(&'a PrintNannyConfig),
    IssueTxt(&'a str),
}

#[get("/")]
async fn index(jar: &CookieJar<'_>) -> Result<Response, Response> {
    let maybe_config = auth::is_auth_valid(jar)?;
    match maybe_config {
        Some(config) => {
            let mut context = Map::new();
            context.insert("config".to_string(), Value::Object(Object::from(&config)));

            let issue_txt = fs::read_to_string(&config.paths.issue_txt)?;
            context.insert("issue_text".to_string(), Value::String(issue_txt));
            Ok(Response::Template(Template::render("index", context)))
        }
        None => Ok(Response::Redirect(Redirect::to("/login"))),
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index]
}
