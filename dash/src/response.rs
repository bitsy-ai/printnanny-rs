use std::collections::HashMap;
use std::convert::From;
use std::fs;

use printnanny_services::config::PrintNannyConfig;
use printnanny_services::error::ServiceError;
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket_dyn_templates::Template;

impl From<serde_json::Error> for Response {
    fn from(error: serde_json::Error) -> Self {
        let msg = format!("Error de/serializing content {:?}", error);
        let config = PrintNannyConfig::new().expect("Failed to read PrintNannyConfig");
        let issue_txt = fs::read_to_string(&config.paths.issue_txt)
            .unwrap_or("Failed to open issue.txt".into());

        let mut context = HashMap::new();
        context.insert("errors", &msg);
        context.insert("issue_txt", &issue_txt);

        error!("{}", &msg);
        Self::Template(Template::render("error", context))
    }
}

impl From<rocket::figment::error::Error> for Response {
    fn from(error: rocket::figment::error::Error) -> Self {
        let msg = format!("Error de/serializing content {:?}", error);
        let config = PrintNannyConfig::new().expect("Failed to read PrintNannyConfig");
        let issue_txt = fs::read_to_string(&config.paths.issue_txt)
            .unwrap_or("Failed to open issue.txt".into());

        let mut context = HashMap::new();
        context.insert("errors", &msg);
        context.insert("issue_txt", &issue_txt);

        error!("{}", &msg);
        Self::Template(Template::render("error", context))
    }
}

impl From<ServiceError> for Response {
    fn from(error: ServiceError) -> Self {
        let msg = format!("Error de/serializing content {:?}", error);
        let config = PrintNannyConfig::new().expect("Failed to read PrintNannyConfig");
        let issue_txt = fs::read_to_string(&config.paths.issue_txt)
            .unwrap_or_else(|_| "Failed to open issue.txt".into());

        let mut context = HashMap::new();
        context.insert("errors", &msg);
        context.insert("issue_txt", &issue_txt);

        error!("{}", &msg);
        Self::Template(Template::render("error", context))
    }
}

impl From<std::io::Error> for Response {
    fn from(error: std::io::Error) -> Self {
        let msg = format!("File I/O error {:?}", error);
        let config = PrintNannyConfig::new().expect("Failed to read PrintNannyConfig");
        let issue_txt = fs::read_to_string(&config.paths.issue_txt)
            .unwrap_or_else(|_| "Failed to open issue.txt".into());
        let mut context = HashMap::new();
        context.insert("errors", &msg);
        context.insert("issue_txt", &issue_txt);
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
