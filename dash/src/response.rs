use std::collections::HashMap;
use std::convert::From;

use printnanny_services::config::PrintNannyConfig;
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket_dyn_templates::Template;

impl From<serde_json::Error> for Response {
    fn from(error: serde_json::Error) -> Self {
        let msg = format!("Error de/serializing content {:?}", error);
        let mut context = HashMap::new();
        context.insert("errors", &msg);
        error!("{}", &msg);
        Self::Template(Template::render("error", context))
    }
}

impl From<rocket::figment::error::Error> for Response {
    fn from(error: rocket::figment::error::Error) -> Self {
        let msg = format!("Error de/serializing content {:?}", error);
        let mut context = HashMap::new();
        context.insert("errors", &msg);
        error!("{}", &msg);
        Self::Template(Template::render("error", context))
    }
}

impl From<printnanny_services::printnanny_api::ServiceError> for Response {
    fn from(error: printnanny_services::printnanny_api::ServiceError) -> Self {
        let msg = format!("Error de/serializing content {:?}", error);
        let mut context = HashMap::new();
        context.insert("errors", &msg);
        error!("{}", &msg);
        Self::Template(Template::render("error", context))
    }
}

#[derive(Debug, Responder)]
pub enum Response {
    PrintNannyConfig(Json<PrintNannyConfig>),
    Template(Template),
    Redirect(Redirect),
}
