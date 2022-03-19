use log::info;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::Template;

use super::auth;
use super::response::Response;
use super::status;

#[get("/")]
async fn index(
    jar: &CookieJar<'_>,
    config_file: &State<auth::PrintNannyConfigFile>,
) -> Result<Response, Response> {
    let health_check = status::HealthCheck::new()?;
    match health_check.firstboot_ok {
        false => Ok(Response::Redirect(Redirect::to("/status"))),
        true => {
            let maybe_config = auth::is_auth_valid(jar, config_file)?;
            match maybe_config {
                Some(config) => {
                    info!("Attaching context to view {:?}", &config);
                    Ok(Response::Template(Template::render("index", config)))
                }
                None => Ok(Response::Redirect(Redirect::to("/login"))),
            }
        }
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index]
}
