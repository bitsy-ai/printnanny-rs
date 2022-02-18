use crate::auth::PrintNannyConfigFile;
use crate::response::Response;
use printnanny_services::config::PrintNannyConfig;
use rocket::serde::json::Json;
use rocket::State;

#[get("/")]
fn get_config(config_file: &State<PrintNannyConfigFile>) -> Result<Response, Response> {
    let config = PrintNannyConfig::new(config_file.0.as_deref())?;
    info!("Rendering config {:?}", config);
    Ok(Response::PrintNannyConfig(Json(config)))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get_config,]
}
