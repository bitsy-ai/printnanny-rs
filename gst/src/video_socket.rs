use std::path::PathBuf;

use anyhow::Result;
use clap::{crate_authors, Arg, ArgMatches, Command};
use gst::prelude::*;

use super::options::SrcOption;
use super::pipeline::GstPipeline;
use printnanny_services::paths::PrintNannyPaths;

pub struct VideoSocketPipeline {
    pub shm_size: u32,
    pub shm_wait_for_connection: bool,
    pub shm_socket: String,

    pub video_height: i32,
    pub video_width: i32,
    pub video_fps: i32,
    pub video_src: SrcOption,
}

impl VideoSocketPipeline {}

impl GstPipeline for VideoSocketPipeline {
    fn clap_command() -> Command<'static> {
        let app_name = "video.sock";
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
                Arg::new("shm_socket")
                    .long("shm-socket")
                    .default_value("/var/run/printnanny/video.sock")
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

        let elements = [&src, &incapsfilter, &queue, &shmsink];
        pipeline.add_many(&elements)?;
        gst::Element::link_many(&elements)?;

        Ok(pipeline)
    }
}

impl Default for VideoSocketPipeline {
    fn default() -> Self {
        let paths = PrintNannyPaths::default();
        return Self {
            shm_size: 134217728, // 128MB
            shm_socket: paths.video_socket().display().to_string(),
            shm_wait_for_connection: true,
            video_height: 480,
            video_width: 640,
            video_fps: 24,
            video_src: SrcOption::Libcamerasrc,
        };
    }
}

impl From<&ArgMatches> for VideoSocketPipeline {
    fn from(args: &ArgMatches) -> Self {
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

        let shm_size: u32 = args.value_of_t("shm_size").expect("--shm-size is required");
        let shm_wait_for_connection: bool = args
            .value_of_t("shm_wait_for_connection")
            .expect("--shm-wait-for-connection is required");
        let shm_socket: String = args
            .value_of_t("shm_socket")
            .expect("--shm-socket is required");

        Self {
            shm_size,
            shm_socket,
            shm_wait_for_connection,
            video_height,
            video_width,
            video_fps,
            video_src,
        }
    }
}
