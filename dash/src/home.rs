use log::{info, warn};
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::Template;

use super::auth;
use super::response::Response;
use crate::auth::PrintNannyConfigFile;
use printnanny_api_client::models::User;
use printnanny_services::config::PrintNannyConfig;

#[get("/")]
async fn index(
    jar: &CookieJar<'_>,
    config_file: &State<PrintNannyConfigFile>,
) -> Result<Response, Response> {
    let cookie = jar.get_private(auth::COOKIE_USER);
    match cookie {
        Some(user_json) => {
            let user: User = serde_json::from_str(user_json.value())?;
            let config = PrintNannyConfig::new(config_file.0.as_deref())?;
            assert_eq!(config.user, Some(user));
            info!("Attaching context to view {:?}", &config);
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
