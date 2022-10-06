use gst::glib;
use gst::prelude::*;

mod imp;

// This enum may be used to control what type of output the dataframe aggregator produces
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "GstDataframeAggOutput")]
pub enum DataframeOutputType {
    #[enum_value(
        name = "Arrow Streaming IPC: outputs the aggregate dataframe in arrow streaming ipc format",
        nick = "arrow-streaming-ipc"
    )]
    ArrowStreamingIpc = 0,
    #[enum_value(
        name = "JSON: output the aggregate dataframe as JSON bytearray",
        nick = "json"
    )]
    Json = 1,
    #[enum_value(
        name = "JSON: output the aggregate dataframe as framed JSON bytearray",
        nick = "json-framed"
    )]
    JsonFramed = 2,
}

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
