use gst::glib;
use gst::prelude::*;

mod imp;

// The public Rust wrapper type for our element
glib::wrapper! {
    pub struct DataframeAgg(ObjectSubclass<imp::DataframeAgg>) @extends gst::Bin, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "dataframe_agg",
        gst::Rank::None,
        DataframeAgg::static_type(),
    )
}
