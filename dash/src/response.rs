use std::fmt;
use std::ops::Deref;
use std::convert::From;

use rocket_dyn_templates::Template;
use rocket::response::{Flash, Redirect};
use thiserror::Error;

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

impl From<services::printnanny_api::ServiceError> for FlashResponse<Redirect> {
    fn from(error: services::printnanny_api::ServiceError) -> Self {
        let msg = format!("Error communicating with PrintNanny API. Please try again in a few minutes. \n {:?}", error);
        error!("{}", &msg);
        Self(Flash::error(Redirect::to("/error"), &msg))
    }
}

impl From<serde_json::Error> for FlashResponse<Redirect> {
    fn from(error: serde_json::Error) -> Self {
        let msg = format!("Error de/serialzing content {:?}", error);
        error!("{}", &msg);
        Self(Flash::error(Redirect::to("/error"), &msg))
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