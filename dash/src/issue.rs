use rocket_dyn_templates::Template;
use serde::{Deserialize, Serialize};
use std::fs;

use super::response::Response;
use printnanny_services::config::PrintNannyConfig;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IssueContext {
    issue_txt: String,
}

#[get("/")]
async fn index() -> Result<Response, Response> {
    let config = PrintNannyConfig::new()?;
    let issue_txt = fs::read_to_string(&config.paths.issue_txt)?;
    let context = IssueContext { issue_txt };
    Ok(Response::Template(Template::render("issue", context)))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index]
}
