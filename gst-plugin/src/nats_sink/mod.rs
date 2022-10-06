use gst::glib;
use gst::prelude::*;

mod imp;

glib::wrapper! {
    pub struct NatsSink(ObjectSubclass<imp::NatsSink>) @extends gst_base::BaseSink, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "nats_sink",
        gst::Rank::None,
        NatsSink::static_type(),
    )
}
