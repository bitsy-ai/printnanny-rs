use std::fmt;
use std::ops::Deref;
use std::convert::From;
use std::collections::HashMap;

use rocket_dyn_templates::Template;
use rocket::response::{Flash, Redirect};
use thiserror::Error;

use crate::error;

#[derive(Error, Debug, Responder)]
pub struct FlashResponse<R>(Flash<R>);

impl<R> Deref for FlashResponse<R> {
    type Target = Flash<R>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<R> fmt::Display for FlashResponse<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<services::printnanny_api::ServiceError> for FlashResponse<Template> {
    fn from(error: services::printnanny_api::ServiceError) -> Self {
        let msg = format!("{:?}", error);
        let mut context = HashMap::new();
        context.insert("errors", &msg);
        error!("{}", &msg);
        Self(Flash::error(Template::render("error", context), &msg))
    }
}

impl From<error::Error> for FlashResponse<Template> {
    fn from(error: error::Error) -> Self {
        let msg = format!("{:?}", error);
        let mut context = HashMap::new();
        context.insert("errors", &msg);
        error!("{}", &msg);
        Self(Flash::error(Template::render("error", context), &msg))
    }
}

impl From<serde_json::Error> for FlashResponse<Template> {
    fn from(error: serde_json::Error) -> Self {
        let msg = format!("Error de/serialzing content {:?}", error);
        let mut context = HashMap::new();
        context.insert("errors", &msg);
        error!("{}", &msg);
        Self(Flash::error(Template::render("error", context), &msg))
    }
}

impl<R> From<Flash<R>> for FlashResponse<R> {
    fn from(call: Flash<R>) -> Self {
        Self(call)
    }
}

#[derive(Debug, Responder)]
pub enum Response {
    Template(Template),
    Redirect(Redirect),
}