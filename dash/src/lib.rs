#[macro_use] extern crate rocket;
use rocket_dyn_templates::Template;
use rocket::response::Redirect;

pub mod auth;

#[derive(Debug, Responder)]
pub enum Response {
    Template(Template),
    Redirect(Redirect),
}

pub struct Config {
    pub base_url: String,
    pub path: String
}