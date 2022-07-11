#[macro_use]
extern crate clap;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use env_logger::Builder;
use git_version::git_version;
use gst::prelude::*;
use log::{error, info, LevelFilter};

use printnanny_gst::options::SrcOption;

pub struct PrintNannyCamApp {
    pub video_height: i32,
    pub video_width: i32,
    pub video_fps: i32,
    pub video_src: SrcOption,
}

impl PrintNannyCamApp {
    pub fn new(args: &ArgMatches) -> Self {
        let video_height: i32 = args
            .value_of_t("video_height")
            .expect("--video-height is required");
        let video_width: i32 = args
            .value_of_t("video_width")
            .expect("--video-width is required");
        let video_fps: i32 = args
            .value_of_t("video_fps")
            .expect("--video-fps is required");
        let video_src: SrcOption = args
            .value_of_t("video_src")
            .expect("--video-src is required");
        Self {
            video_height,
            video_width,
            video_fps,
            video_src,
        }
    }
    pub fn build_pipeline(&self) -> Result<gst::Pipeline> {
        // initialize pipeline, input source, and rtpbin
        let pipeline = gst::Pipeline::new(None);
        let src = gst::ElementFactory::make(&self.video_src.to_string(), Some("video_src"))?;
        let rtpbin = gst::ElementFactory::make("rtpbin", Some("rtpbin0"))?;
        // set input caps
        let incapsfilter = gst::ElementFactory::make("capsfilter", Some("incapsfilter"))?;
        let incaps = gst::Caps::builder("video/x-raw")
            .field("width", &self.video_width)
            .field("height", &self.video_height)
            .field("fps", &self.video_fps)
            .build();
        incapsfilter.set_property("caps", incaps);
        let tee = gst::ElementFactory::make("tee", Some("t0"))?;

        // encode h264 video
        let converter = gst::ElementFactory::make("v4l2convert", None)?;
        let encoder = gst::ElementFactory::make("v4l2h264enc", None)?;
        encoder.set_property_from_str("extra-controls", "controls,repeat_sequence_header=1");
        let encapsfilter = gst::ElementFactory::make("capsfilter", Some("encapsfilter"))?;
        let encaps = gst::Caps::builder("video/x-h264")
            .field("width", &self.video_width)
            .field("height", &self.video_height)
            .field("fps", &self.video_fps)
            .field("level", "level=(string)4")
            .build();
        encapsfilter.set_property("caps", encaps);

        // parse to rtp payload
        let payloader = gst::ElementFactory::make("rtph264pay", None)?;
        // TODO: read janus gateway edge/cloud from settings

        // let config = PrintNannyConfig::new()?;
        // sink to Janus Streaming plugin API (Edge)
        let janus_edge_sink = gst::ElementFactory::make("udpsink", None)?;
        janus_edge_sink.set_property_from_str("host", "127.0.0.1");
        janus_edge_sink.set_property_from_str("port", "5105");

        // TODO: sink to Janus Streaming plugin API (Cloud)

        // sink to PrintNanny Vision service
        let vision_edge_sink = gst::ElementFactory::make("udpsink", None)?;
        vision_edge_sink.set_property_from_str("host", "127.0.0.1");
        vision_edge_sink.set_property_from_str("port", "5205");

        pipeline.add_many(&[
            &src,
            &rtpbin,
            &incapsfilter,
            &converter,
            &encoder,
            &encapsfilter,
            &payloader,
            &tee,
        ]);

        // src -> payload pipeline segment
        gst::Element::link_many(&[
            &src,
            &incapsfilter,
            &converter,
            &encoder,
            &encapsfilter,
            &payloader,
            &tee,
        ])?;
        // tee payloader to each rtp receiver
        let tee_pad = tee
            .request_pad_simple("src_%u")
            .expect(&format!("Failed to get src pad from tee element {:?}", tee));
        let janus_edge_sink_pad = janus_edge_sink.static_pad("sink").expect(&format!(
            "Failed to get sink pad from udpsink element {:?}",
            &janus_edge_sink
        ));
        let vision_edge_sink_pad = vision_edge_sink.static_pad("sink").expect(&format!(
            "Failed to get sink pad from udpsink element {:?}",
            &vision_edge_sink
        ));
        tee_pad.link(&janus_edge_sink_pad)?;
        tee_pad.link(&vision_edge_sink_pad)?;
        Ok(pipeline)
    }

    pub fn run(&self) -> Result<()> {
        let pipeline = self.build_pipeline()?;
        let bus = pipeline
            .bus()
            .expect("Pipeline without bus. Shouldn't happen!");
        pipeline.set_state(gst::State::Playing)?;
        for msg in bus.iter_timed(gst::ClockTime::NONE) {
            use gst::MessageView;
            match msg.view() {
                MessageView::Eos(..) => break,
                MessageView::Error(err) => {
                    error!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                    break;
                }
                MessageView::StateChanged(state_changed) => {
                    // Generate a dot graph of the pipeline to GST_DEBUG_DUMP_DOT_DIR if defined

                    if state_changed.src().map(|s| s == pipeline).unwrap_or(false) {
                        pipeline.debug_to_dot_file(
                            gst::DebugGraphDetails::all(),
                            format!("{:?}-{:?}", &state_changed.old(), &state_changed.current()),
                        );
                    }
                }
                _ => (),
            }
        }
        info!("Setting pipeline {:?} state to Null", pipeline);
        pipeline
            .set_state(gst::State::Null)
            .expect("Unable to set the pipeline to the `Null` state");

        Ok(())
    }
}

fn main() -> Result<()> {
    // include git sha in version, which requires passing a boxed string to clap's .version() builder
    let version = Box::leak(format!("{} {}", crate_version!(), git_version!()).into_boxed_str());
    // parse args
    let app_name = "printnanny-cam";
    let app = Command::new(app_name)
        .author(crate_authors!())
        .about("Encode live video camera stream")
        .version(&version[..])
        // generic app args
        .arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::new("video_src")
                .long("video-src")
                .default_value("libcamerasrc")
                .takes_value(true)
                .possible_values(SrcOption::possible_values())
                .help("Input video source element"),
        )
        .arg(
            Arg::new("video_height")
                .long("video-height")
                .default_value("480")
                .takes_value(true)
                .help("Input video height"),
        )
        .arg(
            Arg::new("video_width")
                .long("video-width")
                .default_value("640")
                .takes_value(true)
                .help("Input video width"),
        )
        .arg(
            Arg::new("video_fps")
                .long("video-fps")
                .default_value("24")
                .takes_value(true)
                .help("Input video frames per second"),
        );
    let app_m = app.get_matches();
    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'printnanny v v v' or 'printnanny vvv' vs 'printnanny v'
    let verbosity = app_m.occurrences_of("v");
    let mut builder = Builder::new();
    match verbosity {
        0 => {
            builder.filter_level(LevelFilter::Warn).init();
            gst::debug_set_default_threshold(gst::DebugLevel::Warning);
        }
        1 => {
            builder.filter_level(LevelFilter::Info).init();
            gst::debug_set_default_threshold(gst::DebugLevel::Info);
        }
        2 => {
            builder.filter_level(LevelFilter::Debug).init();
            gst::debug_set_default_threshold(gst::DebugLevel::Debug);
        }
        _ => {
            gst::debug_set_default_threshold(gst::DebugLevel::Trace);
            builder.filter_level(LevelFilter::Trace).init()
        }
    };

    // Initialize GStreamer first
    gst::init()?;
    let app = PrintNannyCamApp::new(&app_m);
    app.run()?;
    Ok(())
}
