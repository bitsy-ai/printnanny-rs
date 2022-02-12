use crate::auth::PrintNannyConfigFile;
use crate::response::{FlashResponse, Response};
use printnanny_services::config::PrintNannyConfig;
use rocket::serde::json::Json;
use rocket::State;
use rocket_dyn_templates::Template;

#[get("/")]
fn get_config(
    config_file: &State<PrintNannyConfigFile>,
) -> Result<Response, FlashResponse<Template>> {
    let config = PrintNannyConfig::new(config_file.0.as_deref())?;
    info!("Rendering config {:?}", config);
    Ok(Response::PrintNannyConfig(Json(config)))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get_config,]
}
