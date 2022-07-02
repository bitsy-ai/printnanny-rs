use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
#[display(fmt = "Missing gstreamer element {}", _0)]
pub struct MissingElement(#[error(not(source))] pub &'static str);
