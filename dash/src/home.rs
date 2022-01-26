use log::info;
use rocket::form::Context;
use rocket::http::CookieJar;
use rocket::State;
use rocket_dyn_templates::Template;

use super::auth;
use super::response::{FlashResponse, Response};
use printnanny_services::config::PrintNannyConfig;

#[get("/")]
async fn index(
    jar: &CookieJar<'_>,
    config: &State<PrintNannyConfig>,
) -> Result<Response, FlashResponse<Template>> {
    let api_config = jar.get_private(auth::COOKIE_CONFIG);
    match api_config {
        Some(cookie) => {
            let config = serde_json::from_str(cookie.value())?;
            let context = auth::get_context(config).await?;
            info!("Attaching context to view {:?}", context);
            Ok(Response::Template(Template::render("index", context)))
        }
        None => Ok(Response::Template(Template::render(
            "authemail",
            &Context::default(),
        ))),
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index]
}
