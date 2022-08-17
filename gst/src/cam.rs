use anyhow::Result;
use clap::{crate_authors, Arg, ArgMatches, Command};
use gst::prelude::*;
use log::{error, info};

use super::options::SrcOption;
use printnanny_api_client::models;
use printnanny_services::config::PrintNannyConfig;

pub struct PrintNannyCam {
    pub video_height: i32,
    pub video_width: i32,
    pub video_fps: i32,
    pub video_src: SrcOption,
}

impl PrintNannyCam {
    pub fn clap_command() -> Command<'static> {
        let app_name = "cam";
        let app = Command::new(app_name)
            .author(crate_authors!())
            .about("Encode live video camera stream")
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
        app
    }
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
            .field("level", "4")
            .build();
        encapsfilter.set_property("caps", encaps);

        // parse to rtp payload
        let payloader = gst::ElementFactory::make("rtph264pay", None)?;
        // TODO: read janus gateway edge/cloud from settings

        let config = PrintNannyConfig::new()?;
        let device_settings: models::PiSettings = *config
            .pi
            .clone()
            .expect("PrintNannyConfig.pi is not set")
            .settings
            .expect("PrintNannyConfig.pi.settings is not set");

        let webrtc_cloud_config = *config
            .pi
            .clone()
            .expect("PrintNannyConfig.pi is not set")
            .webrtc_cloud
            .expect("PrintNannyConfig.pi.webrtc_edge is not set");
        let webrtc_edge_config = *config
            .pi
            .expect("PrintNannyConfig.pi is not set")
            .webrtc_edge
            .expect("PrintNannyConfig.pi.webrtc_edge is not set");

        // sink to Janus Streaming plugin API (Cloud) if cloud_video_enabled
        if device_settings.cloud_video_enabled.unwrap() {
            let webrtc_cloud_host = webrtc_cloud_config.rtp_domain;
            let webrtc_cloud_port = webrtc_cloud_config
                .rtp_port
                .expect("PrintNannyConfig.janus.cloud.rtp_port is not set")
                .to_string();
            let webrtc_cloud_queue = gst::ElementFactory::make("queue2", Some("januscloud_queue"))?;
            let webrtc_cloud_sink = gst::ElementFactory::make("udpsink", Some("januscloud_sink"))?;
            webrtc_cloud_sink.set_property_from_str("host", &webrtc_cloud_host);
            webrtc_cloud_sink.set_property_from_str("port", &webrtc_cloud_port);
            pipeline.add_many(&[&webrtc_cloud_queue, &webrtc_cloud_sink])?;
            let webrtc_cloud_tee_pad = tee
                .request_pad_simple("src_%u")
                .unwrap_or_else(|| panic!("Failed to get src pad from tee element {:?}", tee));
            let webrtc_cloud_q_pad = webrtc_cloud_queue.static_pad("sink").unwrap_or_else(|| {
                panic!(
                    "Failed to get sink pad from queue element {:?}",
                    &webrtc_cloud_queue
                )
            });
            webrtc_cloud_tee_pad.link(&webrtc_cloud_q_pad)?;
        }

        // sink to Janus Streaming plugin API (Edge)
        let webrtc_edge_port = webrtc_edge_config.rtp_port.unwrap_or(5105).to_string();
        let webrtc_edge_queue = gst::ElementFactory::make("queue2", Some("janusedge_queue"))?;
        let webrtc_edge_sink = gst::ElementFactory::make("udpsink", Some("janusedge_udpsink"))?;
        webrtc_edge_sink.set_property_from_str("host", "127.0.0.1");
        webrtc_edge_sink.set_property_from_str("port", &webrtc_edge_port);

        // sink to PrintNanny Vision service
        let vision_edge_queue = gst::ElementFactory::make("queue2", None)?;
        let vision_edge_sink = gst::ElementFactory::make("udpsink", None)?;
        vision_edge_sink.set_property_from_str("host", "127.0.0.1");
        vision_edge_sink.set_property_from_str("port", "5205");

        // tee payloader to each rtp receiver
        let webrtc_edge_tee_pad = tee
            .request_pad_simple("src_%u")
            .unwrap_or_else(|| panic!("Failed to get src pad from tee element {:?}", tee));

        let webrtc_edge_q_pad = webrtc_edge_queue.static_pad("sink").unwrap_or_else(|| {
            panic!(
                "Failed to get sink pad from queue element {:?}",
                &webrtc_edge_queue
            )
        });
        webrtc_edge_tee_pad.link(&webrtc_edge_q_pad)?;

        let vision_edge_tee_pad = tee
            .request_pad_simple("src_%u")
            .unwrap_or_else(|| panic!("Failed to get src pad from tee element {:?}", tee));
        let vision_edge_q_pad = vision_edge_sink.static_pad("sink").unwrap_or_else(|| {
            panic!(
                "Failed to get sink pad from queue element {:?}",
                &vision_edge_queue
            )
        });
        vision_edge_tee_pad.link(&vision_edge_q_pad)?;

        pipeline.add_many(&[
            &src,
            &incapsfilter,
            &converter,
            &encoder,
            &encapsfilter,
            &payloader,
            &tee,
            &webrtc_edge_queue,
            &webrtc_edge_sink,
            &vision_edge_queue,
            &vision_edge_sink,
        ])?;

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
        // queue -> sink pipeline segments
        Ok(pipeline)
    }

    pub fn run(&self) -> Result<()> {
        gst::init()?;
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
