use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket_dyn_templates::Template;
use serde::{Deserialize, Serialize};

use super::auth;
use super::response::Response;
use printnanny_services::config::PrintNannyConfig;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashContext {
    config: PrintNannyConfig,
}

#[get("/")]
async fn index(jar: &CookieJar<'_>) -> Result<Response, Response> {
    let maybe_config = auth::is_auth_valid(jar).await?;
    match maybe_config {
        Some(config) => {
            let context = DashContext { config };
            Ok(Response::Template(Template::render("index", context)))
        }
        None => Ok(Response::Redirect(Redirect::to("/login"))),
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index]
}
