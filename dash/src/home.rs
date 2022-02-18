use log::{info, warn};
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket_dyn_templates::Template;

use super::auth;
use super::response::Response;
use printnanny_services::config::PrintNannyConfig;

#[get("/")]
async fn index(jar: &CookieJar<'_>) -> Result<Response, Response> {
    let api_config = jar.get_private(auth::COOKIE_USER);
    match api_config {
        Some(cookie) => {
            let config: PrintNannyConfig = serde_json::from_str(cookie.value())?;
            info!("Attaching context to view {:?}", config);
            Ok(Response::Template(Template::render("index", config)))
        }
        None => {
            warn!(
                "Failed to read auth::COOKIE_USER={:?}, redirecting to /login",
                auth::COOKIE_USER
            );
            Ok(Response::Redirect(Redirect::to("/login")))
        }
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index]
}
