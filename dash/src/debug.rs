use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket::serde::json::Json;

use super::auth;
use crate::response::Response;
#[get("/")]
fn get_config(jar: &CookieJar<'_>) -> Result<Response, Response> {
    let maybe_config = auth::is_auth_valid(jar)?;
    match maybe_config {
        Some(config) => Ok(Response::PrintNannyConfig(Json(config))),
        None => Ok(Response::Redirect(Redirect::to("/login"))),
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get_config,]
}
