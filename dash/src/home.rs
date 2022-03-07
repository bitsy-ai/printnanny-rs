use log::{info, warn};
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::Template;

use super::auth;
use super::response::Response;
use crate::auth::PrintNannyConfigFile;
use printnanny_api_client::models::User;
use printnanny_services::config::PrintNannyConfig;

pub fn is_auth_valid(jar: &CookieJar<'_>) -> Result<Option<PrintNannyConfig>> {
    let cookie = jar.get_private(auth::COOKIE_USER);
    match cookie {
        Some(user_json) => {
            let user: User = serde_json::from_str(user_json.value())?;
            let config = PrintNannyConfig::new(config_file.0.as_deref())?;

            // if config + cookie mismatch, nuke cookie (profile switch in developer mode)
            if config.user != Some(user.clone()) {
                warn!("config.user {:?} did not match COOKIE_USER {:?}, deleting cookie to force re-auth", config.user, &user);
                jar.remove_private(Cookie::named(auth::COOKIE_USER));
                config.remove_license()?;
                Ok(None)
            } else {
                info!("Auth success! COOKIE_USER matches config.user");
                info!("Attaching context to view {:?}", &config);
                Ok(Response::Template(Template::render("index", config)))
            }
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

            // if config + cookie mismatch, nuke cookie (profile switch in developer mode)
            if config.user != Some(user.clone()) {
                warn!("config.user {:?} did not match COOKIE_USER {:?}, deleting cookie to force re-auth", config.user, &user);
                jar.remove_private(Cookie::named(auth::COOKIE_USER));
                Ok(Response::Redirect(Redirect::to("/login")))
            } else {
                info!("Auth success! COOKIE_USER matches config.user");
                info!("Attaching context to view {:?}", &config);
                Ok(Response::Template(Template::render("index", config)))
            }
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
