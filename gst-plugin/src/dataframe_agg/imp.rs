use std::sync::{Arc, Mutex};

use gst::glib;
use gst::prelude::*;
use gst::subclass::prelude::*;

use once_cell::sync::Lazy;
use polars::prelude::*;

use super::DataframeOutputType;
use crate::ipc::{
    dataframe_to_arrow_streaming_ipc_message, dataframe_to_framed_json_bytearray,
    dataframe_to_json_bytearray,
};

static CAT: Lazy<gst::DebugCategory> = Lazy::new(|| {
    gst::DebugCategory::new(
        "dataframe_agg",
        gst::DebugColorFlags::empty(),
        Some("PrintNanny Dataframe aggregator"),
    )
});

const DEFAULT_OUTPUT_TYPE: DataframeOutputType = DataframeOutputType::ArrowStreamingIpc;

const DEFAULT_MAX_SIZE_DURATION: &str = "30s";
const DEFAULT_MAX_SIZE_BUFFERS: u64 = 900; // approx 1 minute of buffer frames @ 15fps
const DEFAULT_WINDOW_INTERVAL: &str = "1s";
const DEFAULT_WINDOW_PERIOD: &str = "1s";
const DEFAULT_WINDOW_OFFSET: &str = "0s";
const DEFAULT_SCORE_THRESHOLD: f32 = 0.5;
const DEFAULT_DDOF: u8 = 0; // delta degrees of freedom, used in std dev calculation. divisor = N - ddof, where N is the number of element in the set
const DEFAULT_WINDOW_TRUNCATE: bool = false;
const DEFAULT_WINDOW_INCLUDE_BOUNDARIES: bool = true;

struct State {
    dataframe: LazyFrame,
}

impl Default for State {
    fn default() -> Self {
        let x0: Vec<f32> = vec![];
        let y0: Vec<f32> = vec![];
        let y1: Vec<f32> = vec![];
        let x1: Vec<f32> = vec![];
        let classes: Vec<i32> = vec![];
        let scores: Vec<f32> = vec![];
        let ts: Vec<i64> = vec![];

        let dataframe = df!(
            "detection_boxes_x0" => x0,
            "detection_boxes_y0" => y0,
            "detection_boxes_x1" => x1,
            "detection_boxes_y1" =>y1,
            "detection_classes" => classes,
            "detection_scores" => scores,
            "ts" => ts
        )
        .expect("Failed to initialize dataframe")
        .lazy();
        Self { dataframe }
    }
}

struct Settings {
    filter_threshold: f32,
    ddof: u8,
    output_type: DataframeOutputType,
    max_size_duration: String,
    max_size_buffers: u64,
    window_interval: String,
    window_period: String,
    window_offset: String,
    window_truncate: bool,
    window_include_boundaries: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ddof: DEFAULT_DDOF,
            output_type: DEFAULT_OUTPUT_TYPE,
            filter_threshold: DEFAULT_SCORE_THRESHOLD,
            max_size_duration: DEFAULT_MAX_SIZE_DURATION.into(),
            max_size_buffers: DEFAULT_MAX_SIZE_BUFFERS,
            window_interval: DEFAULT_WINDOW_INTERVAL.into(),
            window_period: DEFAULT_WINDOW_PERIOD.into(),
            window_offset: DEFAULT_WINDOW_OFFSET.into(),
            window_truncate: DEFAULT_WINDOW_TRUNCATE,
            window_include_boundaries: DEFAULT_WINDOW_INCLUDE_BOUNDARIES,
        }
    }
}

#[derive(Clone)]
pub struct DataframeAgg {
    settings: Arc<Mutex<Settings>>,
    state: Arc<Mutex<State>>,
    sinkpad: gst::Pad,
    srcpad: gst::Pad,
}

impl DataframeAgg {
    fn drain(&self) -> Result<(), gst::ErrorMessage> {
        Ok(())
    }

    // Called whenever an event arrives on the sink pad. It has to be handled accordingly and in
    // most cases has to be either passed to Pad::event_default() on this pad for default handling,
    // or Pad::push_event() on all pads with the opposite direction for direct forwarding.
    // Here we just pass through all events directly to the source pad.
    //
    // See the documentation of gst::Event and gst::EventRef to see what can be done with
    // events, and especially the gst::EventView type for inspecting events.
    fn sink_event(
        &self,
        pad: &gst::Pad,
        _element: &super::DataframeAgg,
        event: gst::Event,
    ) -> bool {
        gst::log!(CAT, obj: pad, "Handling event {:?}", event);
        self.srcpad.push_event(event)
    }

    // Called whenever an event arrives on the source pad. It has to be handled accordingly and in
    // most cases has to be either passed to Pad::event_default() on the same pad for default
    // handling, or Pad::push_event() on all pads with the opposite direction for direct
    // forwarding.
    // Here we just pass through all events directly to the sink pad.
    //
    // See the documentation of gst::Event and gst::EventRef to see what can be done with
    // events, and especially the gst::EventView type for inspecting events.
    fn src_event(&self, pad: &gst::Pad, _element: &super::DataframeAgg, event: gst::Event) -> bool {
        gst::log!(CAT, obj: pad, "Handling event {:?}", event);
        self.sinkpad.push_event(event)
    }

    // Called whenever a query is sent to the source pad. It has to be answered if the element can
    // handle it, potentially by forwarding the query first to the peer pads of the pads with the
    // opposite direction, or false has to be returned. Default handling can be achieved with
    // Pad::query_default() on this pad and forwarding with Pad::peer_query() on the pads with the
    // opposite direction.
    // Here we just forward all queries directly to the sink pad's peers.
    //
    // See the documentation of gst::Query and gst::QueryRef to see what can be done with
    // queries, and especially the gst::QueryView type for inspecting and modifying queries.
    fn src_query(
        &self,
        pad: &gst::Pad,
        _element: &super::DataframeAgg,
        query: &mut gst::QueryRef,
    ) -> bool {
        gst::log!(CAT, obj: pad, "Handling query {:?}", query);
        self.sinkpad.peer_query(query)
    }

    // Called whenever a query is sent to the sink pad. It has to be answered if the element can
    // handle it, potentially by forwarding the query first to the peer pads of the pads with the
    // opposite direction, or false has to be returned. Default handling can be achieved with
    // Pad::query_default() on this pad and forwarding with Pad::peer_query() on the pads with the
    // opposite direction.
    // Here we just forward all queries directly to the source pad's peers.
    //
    // See the documentation of gst::Query and gst::QueryRef to see what can be done with
    // queries, and especially the gst::QueryView type for inspecting and modifying queries.
    fn sink_query(
        &self,
        pad: &gst::Pad,
        _element: &super::DataframeAgg,
        query: &mut gst::QueryRef,
    ) -> bool {
        gst::log!(CAT, obj: pad, "Handling query {:?}", query);
        self.srcpad.peer_query(query)
    }

    fn sink_chain(
        &self,
        pad: &gst::Pad,
        element: &super::DataframeAgg,
        buffer: gst::Buffer,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        gst::log!(CAT, obj: pad, "Handling buffer {:?}", buffer);

        let mut state = self.state.lock().unwrap();
        let settings = self.settings.lock().unwrap();

        let cursor = buffer.into_cursor_readable();

        let reader = IpcStreamReader::new(cursor);
        let df = reader
            .finish()
            .expect("Failed to deserialize Arrow IPC Stream")
            .lazy();

        let max_duration = Duration::parse(&settings.max_size_duration);
        state.dataframe = concat(vec![state.dataframe.clone(), df], true, true).map_err(|err| {
            gst::element_error!(
                element,
                gst::ResourceError::Read,
                ["Failed to merge dataframes: {}", err]
            );
            gst::FlowError::Error
        })?;

        let group_options = DynamicGroupOptions {
            index_column: "ts".to_string(),
            every: Duration::parse(&settings.window_interval),
            period: Duration::parse(&settings.window_period),
            offset: Duration::parse(&settings.window_offset),
            closed_window: ClosedWindow::Left,
            truncate: false,
            include_boundaries: true,
        };
        println!("{:?}", &state.dataframe.clone().collect());

        let mut windowed_df = state
            .dataframe
            .clone()
            .filter(col("detection_scores").gt(settings.filter_threshold))
            .filter(col("ts").gt_eq(col("ts").max() - lit(max_duration.nanoseconds())))
            .sort(
                "ts",
                SortOptions {
                    descending: false,
                    nulls_last: false,
                },
            )
            .groupby_dynamic([col("detection_classes")], group_options)
            .agg([
                col("detection_scores")
                    .filter(col("detection_classes").eq(0))
                    .count()
                    .alias("nozzle__count"),
                col("detection_scores")
                    .filter(col("detection_classes").eq(0))
                    .mean()
                    .alias("nozzle__mean"),
                col("detection_scores")
                    .filter(col("detection_classes").eq(0))
                    .std(settings.ddof)
                    .alias("nozzle__std"),
                col("detection_scores")
                    .filter(col("detection_classes").eq(1))
                    .count()
                    .alias("adhesion__count"),
                col("detection_scores")
                    .filter(col("detection_classes").eq(1))
                    .mean()
                    .alias("adhesion__mean"),
                col("detection_scores")
                    .filter(col("detection_classes").eq(1))
                    .std(settings.ddof)
                    .alias("adhesion__std"),
                col("detection_scores")
                    .filter(col("detection_classes").eq(2))
                    .count()
                    .alias("spaghetti__count"),
                col("detection_scores")
                    .filter(col("detection_classes").eq(2))
                    .mean()
                    .alias("spaghetti__mean"),
                col("detection_scores")
                    .filter(col("detection_classes").eq(2))
                    .std(settings.ddof)
                    .alias("spaghetti__std"),
                col("detection_scores")
                    .filter(col("detection_classes").eq(3))
                    .count()
                    .alias("print__count"),
                col("detection_scores")
                    .filter(col("detection_classes").eq(3))
                    .mean()
                    .alias("print__mean"),
                col("detection_scores")
                    .filter(col("detection_classes").eq(3))
                    .std(settings.ddof)
                    .alias("print__std"),
                col("detection_scores")
                    .filter(col("detection_classes").eq(4))
                    .count()
                    .alias("raft__count"),
                col("detection_scores")
                    .filter(col("detection_classes").eq(4))
                    .mean()
                    .alias("raft__mean"),
                col("detection_scores")
                    .filter(col("detection_classes").eq(4))
                    .std(settings.ddof)
                    .alias("raft__std"),
            ])
            .collect()
            .map_err(|err| {
                gst::element_error!(
                    element,
                    gst::StreamError::Decode,
                    ["Failed window/aggregate dataframes {}", err]
                );
                gst::FlowError::Error
            })?;

        let output_buffer = match settings.output_type {
            DataframeOutputType::ArrowStreamingIpc => {
                dataframe_to_arrow_streaming_ipc_message(&mut windowed_df, None).map_err(|err| {
                    gst::element_error!(
                        element,
                        gst::StreamError::Decode,
                        ["Failed to serialize arrow ipc streaming msg: {:?}", err]
                    );
                    gst::FlowError::Error
                })?
            }
            DataframeOutputType::Json => {
                dataframe_to_json_bytearray(&mut windowed_df).map_err(|err| {
                    gst::element_error!(
                        element,
                        gst::StreamError::Decode,
                        ["Failed to serialize json from dataframe: {:?}", err]
                    );
                    gst::FlowError::Error
                })?
            }
            DataframeOutputType::JsonFramed => dataframe_to_framed_json_bytearray(&mut windowed_df)
                .map_err(|err| {
                    gst::element_error!(
                        element,
                        gst::StreamError::Decode,
                        ["Failed to serialize framed json from dataframe: {:?}", err]
                    );
                    gst::FlowError::Error
                })?,
        };

        self.srcpad.push(gst::Buffer::from_slice(output_buffer))
    }
}

#[glib::object_subclass]
impl ObjectSubclass for DataframeAgg {
    const NAME: &'static str = "DataframeAgg";
    type Type = super::DataframeAgg;
    type ParentType = gst::Element;

    // Called when a new instance is to be created. We need to return an instance
    // of our struct here and also get the class struct passed in case it's needed
    fn with_class(klass: &Self::Class) -> Self {
        let templ = klass.pad_template("src").unwrap();
        let srcpad = gst::Pad::builder_with_template(&templ, Some("src"))
            .event_function(|pad, parent, event| {
                DataframeAgg::catch_panic_pad_function(
                    parent,
                    || false,
                    |identity, element| identity.src_event(pad, element, event),
                )
            })
            .query_function(|pad, parent, query| {
                DataframeAgg::catch_panic_pad_function(
                    parent,
                    || false,
                    |identity, element| identity.src_query(pad, element, query),
                )
            })
            .build();

        let templ = klass.pad_template("sink").unwrap();
        let sinkpad = gst::Pad::builder_with_template(&templ, Some("sink"))
            .chain_function(|pad, parent, buffer| {
                DataframeAgg::catch_panic_pad_function(
                    parent,
                    || Err(gst::FlowError::Error),
                    |parse, element| parse.sink_chain(pad, element, buffer),
                )
            })
            .event_function(|pad, parent, event| {
                DataframeAgg::catch_panic_pad_function(
                    parent,
                    || false,
                    |parse, element| parse.sink_event(pad, element, event),
                )
            })
            .query_function(|pad, parent, query| {
                DataframeAgg::catch_panic_pad_function(
                    parent,
                    || false,
                    |identity, element| identity.sink_query(pad, element, query),
                )
            })
            .build();
        // Return an instance of our struct
        Self {
            sinkpad,
            srcpad,
            state: Arc::new(Mutex::new(State::default())),
            settings: Arc::new(Mutex::new(Settings::default())),
        }
    }
}

impl ObjectImpl for DataframeAgg {
    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);
        obj.add_pad(&self.sinkpad).unwrap();
        obj.add_pad(&self.srcpad).unwrap();
    }
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpecUInt64::builder("max-size-buffers")
                    .nick("Max Size Buffers")
                    .blurb("Max number of buffers to perform windowed aggregations over")
                    .default_value(DEFAULT_MAX_SIZE_BUFFERS)
                    .build(),
                glib::ParamSpecString::builder("max-size-duration")
                    .nick("Max Size Buffers")
                    .blurb("Max buffer duration to perform windowed aggregations over")
                    .default_value(DEFAULT_MAX_SIZE_DURATION)
                    .build(),
                glib::ParamSpecString::builder("window-interval")
                    .nick("Window Interval")
                    .blurb("Interval between window occurrences")
                    .default_value(DEFAULT_WINDOW_INTERVAL)
                    .build(),
                glib::ParamSpecString::builder("window-period")
                    .nick("Window Period")
                    .blurb("Length/duration of window")
                    .default_value(DEFAULT_WINDOW_PERIOD)
                    .build(),
                glib::ParamSpecString::builder("window-offset")
                    .nick("Window Offset")
                    .blurb("Offset window calculation by this amount")
                    .default_value(DEFAULT_WINDOW_OFFSET)
                    .build(),
                glib::ParamSpecBoolean::builder("window-truncate")
                    .nick("Truncate window")
                    .blurb("Truncate window")
                    .default_value(DEFAULT_WINDOW_TRUNCATE)
                    .build(),
                glib::ParamSpecBoolean::builder("window-include-boundaries")
                    .nick("Window Include Boundaries")
                    .blurb("Include _lower_boundary and _upper_boundary columns in windowed dataframe projection")
                    .default_value(DEFAULT_WINDOW_INCLUDE_BOUNDARIES)
                    .build(),
                glib::ParamSpecFloat::builder("filter-threshold")
                    .nick("Filter Threshold")
                    .blurb("Filter observations where detection_score is below threshold. Float between 0 - 1")
                    .default_value(DEFAULT_SCORE_THRESHOLD)
                    .build(),
                glib::ParamSpecUInt::builder("ddof")
                    .nick("Delta Degrees of Freedom")
                    .blurb("Delta degrees of freedom modifier, used in standard deviation and variance calculations")
                    .default_value(DEFAULT_DDOF as u32)
                    .build(),
                glib::ParamSpecEnum::builder::<DataframeOutputType>("output-type", DEFAULT_OUTPUT_TYPE)
                    .nick("Output Format Type")
                    .blurb("Format of output buffer")
                    .build(),
            ]
        });

        PROPERTIES.as_ref()
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        let settings = self.settings.lock().unwrap();
        match pspec.name() {
            "ddof" => settings.ddof.to_value(),
            "output-type" => settings.output_type.to_value(),
            "filter-threshold" => settings.filter_threshold.to_value(),
            "max-size-buffers" => settings.max_size_buffers.to_value(),
            "max-size-duration" => settings.max_size_duration.to_value(),
            "window-interval" => settings.window_interval.to_value(),
            "window-period" => settings.window_period.to_value(),
            "window-offset" => settings.window_offset.to_value(),
            "window-truncate" => settings.window_truncate.to_value(),
            "window-include-boundaries" => settings.window_include_boundaries.to_value(),
            _ => unimplemented!(),
        }
    }

    fn set_property(
        &self,
        _obj: &Self::Type,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        let mut settings = self.settings.lock().unwrap();

        match pspec.name() {
            "ddof" => {
                settings.ddof = value.get::<u8>().expect("type checked upstream");
            }
            "output-type" => {
                settings.output_type = value
                    .get::<DataframeOutputType>()
                    .expect("type checked upstream");
            }
            "filter-threshold" => {
                settings.filter_threshold = value.get::<f32>().expect("type checked upstream");
            }
            "max-size-buffers" => {
                settings.max_size_buffers = value.get::<u64>().expect("type checked upstream");
            }
            "max-size-duration" => {
                settings.max_size_duration = value.get::<String>().expect("type checked upstream");
            }
            "window-interval" => {
                settings.window_interval = value.get::<String>().expect("type checked upstream");
            }
            "window-period" => {
                settings.window_period = value.get::<String>().expect("type checked upstream");
            }
            "window-offset" => {
                settings.window_offset = value.get::<String>().expect("type checked upstream");
            }
            "window-truncate" => {
                settings.window_truncate = value.get::<bool>().expect("type checked upstream");
            }
            "window-include-boundaries" => {
                settings.window_include_boundaries =
                    value.get::<bool>().expect("type checked upstream");
            }
            _ => unimplemented!(),
        }
    }
}

impl GstObjectImpl for DataframeAgg {}

impl ElementImpl for DataframeAgg {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static ELEMENT_METADATA: Lazy<gst::subclass::ElementMetadata> = Lazy::new(|| {
            gst::subclass::ElementMetadata::new(
                "PrintNanny QC Dataframe aggregator",
                "Filter/Agg",
                "Aggregate windowed dataframes and calculate quality score",
                "Leigh Johnson <leigh@printnanny.ai>",
            )
        });

        Some(&*ELEMENT_METADATA)
    }
    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: Lazy<Vec<gst::PadTemplate>> = Lazy::new(|| {
            let caps = gst::Caps::new_any();

            let sink_pad_template = gst::PadTemplate::new(
                "sink",
                gst::PadDirection::Sink,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();

            let caps = gst::Caps::new_any();
            let src_pad_template = gst::PadTemplate::new(
                "src",
                gst::PadDirection::Src,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();

            vec![src_pad_template, sink_pad_template]
        });

        PAD_TEMPLATES.as_ref()
    }

    // Called whenever the state of the element should be changed. This allows for
    // starting up the element, allocating/deallocating resources or shutting down
    // the element again.
    fn change_state(
        &self,
        element: &Self::Type,
        transition: gst::StateChange,
    ) -> Result<gst::StateChangeSuccess, gst::StateChangeError> {
        gst::trace!(CAT, obj: element, "Changing state {:?}", transition);

        // Call the parent class' implementation of ::change_state()
        self.parent_change_state(element, transition)
    }
}
