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
        FlashResponse(Flash::error(Redirect::to(format!("/login/{}", &email)), "Please enter verification code"))
    }
}

#[derive(Debug, Responder)]
pub enum Response {
    Template(Template),
    Redirect(Redirect),
}