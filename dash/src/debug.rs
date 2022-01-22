use rocket::serde::json::Json;
use rocket::State;
use std::convert::TryInto;

use crate::response::Response;
use printnanny_services::config::PrintNannyConfig;

#[get("/")]
fn get_config(config: &State<PrintNannyConfig>) -> Response {
    info!("Rendering config {:?}", config);
    let c = config.inner().clone();
    Response::PrintNannyConfig(Json(c))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get_config,]
}
