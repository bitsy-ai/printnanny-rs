use std::fs;
use std::process;

use anyhow::Result;
use clap::{crate_authors, value_parser, Arg, ArgMatches, Command};
use gst::prelude::*;
use log::{error, warn};

use printnanny_services::config::PrintNannyConfig;

use super::options::SrcOption;
use super::pipeline::GstPipeline;

#[derive(Debug, Clone, PartialEq)]
pub struct VideoSocketPipeline {
    pub shm_size: u32,
    pub shm_wait_for_connection: bool,
    pub shm_socket: String,
    pub shm_sync: bool,

    pub video_height: i32,
    pub video_width: i32,
    pub video_fps: i32,
    pub video_src: SrcOption,
}

impl VideoSocketPipeline {}

impl GstPipeline for VideoSocketPipeline {
    fn clap_command() -> Command<'static> {
        let app_name = "video.socket";
        let app = Command::new(app_name)
            .author(crate_authors!())
            .about("Write camera src to shared memory: https://gstreamer.freedesktop.org/documentation/shm/shmsink.html?gi-language=c")
            .arg(
                Arg::new("v")
                    .short('v')
                    .multiple_occurrences(true)
                    .help("Sets the level of verbosity"),
            )
            .arg(
                Arg::new("shm_sync")
                .long("shm-sync")
                .help("Set sync property on shmsink https://gstreamer.freedesktop.org/documentation/base/gstbasesink.html?gi-language=c")
            )
            .arg(
                Arg::new("shm_socket")
                    .long("shm-socket")
                    .takes_value(true)
                    .help("shmsink socket path: https://gstreamer.freedesktop.org/documentation/shm/shmsink.html?gi-language=c")
            )
            .arg(
                Arg::new("shm_wait_for_connection")
                    .long("shm-wait-for-connection")
                    .takes_value(true)
                    .default_value("true")
            )
            .arg(
                Arg::new("shm_size")
                    .long("shm-size")
                    .takes_value(true)
                    .default_value("134217728")
            )
            .arg(
                Arg::new("video_src")
                    .long("video-src")
                    .default_value("libcamerasrc")
                    .takes_value(true)
                    .value_parser(value_parser!(SrcOption))
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

    fn on_sigint(&self) -> () {
        warn!("SIGINT received, removing {}", self.shm_socket);
        fs::remove_file(&self.shm_socket)
            .unwrap_or_else(|_| error!("Failed to delete file {}", &self.shm_socket));
        process::exit(0)
    }

    fn build_pipeline(&self) -> Result<gst::Pipeline> {
        // initialize pipeline
        let pipeline = gst::Pipeline::new(None);

        // make input src element
        let src = gst::ElementFactory::make(&self.video_src.to_string(), Some("video_src"))?;

        // set input caps
        let incapsfilter = gst::ElementFactory::make("capsfilter", Some("incapsfilter"))?;
        let incaps = gst::Caps::builder("video/x-raw")
            .field("width", &self.video_width)
            .field("height", &self.video_height)
            .field("framerate", gst::Fraction::new(0, 1))
            .field("format", "RGB")
            .build();
        incapsfilter.set_property("caps", incaps);

        // make queue element
        let queue = gst::ElementFactory::make("queue", None)?;

        // make shmsink element
        let shmsink = gst::ElementFactory::make("shmsink", None)?;
        shmsink.set_property_from_str("socket-path", &self.shm_socket);
        shmsink.set_property("wait-for-connection", &self.shm_wait_for_connection);
        shmsink.set_property("shm-size", &self.shm_size);

        if self.shm_sync {
            shmsink.set_property("sync", &self.shm_sync);
        }

        let elements = [&src, &incapsfilter, &queue, &shmsink];
        pipeline.add_many(&elements)?;
        gst::Element::link_many(&elements)?;

        Ok(pipeline)
    }
}

impl Default for VideoSocketPipeline {
    fn default() -> Self {
        let config = PrintNannyConfig::new().expect("Failed to initialize PrintNannyConfig");

        return Self {
            shm_size: 134217728, // 128MB
            shm_socket: config.paths.video_socket().display().to_string(),
            shm_wait_for_connection: true,
            shm_sync: false,
            video_height: 480,
            video_width: 640,
            video_fps: 24,
            video_src: SrcOption::Libcamerasrc,
        };
    }
}

impl From<&ArgMatches> for VideoSocketPipeline {
    fn from(args: &ArgMatches) -> Self {
        let defaults = VideoSocketPipeline::default();

        let video_height: i32 = args
            .value_of_t("video_height")
            .unwrap_or_else(|_| defaults.video_height);

        let video_width: i32 = args
            .value_of_t("video_width")
            .unwrap_or_else(|_| defaults.video_width);

        let video_fps: i32 = args
            .value_of_t("video_fps")
            .unwrap_or_else(|_| defaults.video_fps);

        let video_src: &SrcOption = args
            .get_one::<SrcOption>("video_src")
            .unwrap_or_else(|| &defaults.video_src);

        let shm_size: u32 = args
            .value_of_t("shm_size")
            .unwrap_or_else(|_| defaults.shm_size);

        let shm_wait_for_connection: bool = args
            .value_of_t("shm_wait_for_connection")
            .unwrap_or_else(|_| defaults.shm_wait_for_connection);

        let shm_socket: String = args
            .value_of_t("shm_socket")
            .unwrap_or_else(|_| defaults.shm_socket);

        let shm_sync: bool = args.is_present("shm_sync");

        Self {
            shm_size,
            shm_socket,
            shm_sync,
            shm_wait_for_connection,
            video_height,
            video_width,
            video_fps,
            video_src: *video_src,
        }
    }
}
