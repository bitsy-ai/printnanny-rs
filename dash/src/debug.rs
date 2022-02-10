use crate::response::Response;
use printnanny_services::config::PrintNannyConfig;
use rocket::serde::json::Json;
use rocket::State;

#[get("/")]
fn get_config(config: &State<PrintNannyConfig>) -> Response {
    info!("Rendering config {:?}", config);
    let c = config.inner().clone();
    Response::PrintNannyConfig(Json(c))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get_config,]
}
