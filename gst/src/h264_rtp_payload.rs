use anyhow::Result;
use std::fs;
use std::process;
use clap::{crate_authors, Arg, ArgMatches, Command, value_parser};
use gst::prelude::*;

use log::{ warn, error};

use printnanny_services::config::PrintNannyConfig;

use super::options::SrcOption;
use super::pipeline::GstPipeline;


#[derive(Debug, Clone, PartialEq)]
pub struct H264EncoderPipeline {
    pub video_height: i32,
    pub video_width: i32,
    pub video_src: SrcOption,
    pub h264_level: String,
    pub shm_src_socket: String,
    pub shm_sink_socket: String,
    pub shm_size: u32,
    pub shm_wait_for_connection: bool,
    pub shm_sync: bool
}

impl GstPipeline for H264EncoderPipeline {
    fn clap_command() -> Command<'static> {
        let app_name = "h264-rtp-payload.socket";
        let app = Command::new(app_name)
            .author(crate_authors!())
            .about("Encode h264 video and RTP payloader, write to shared memory socket")
            .arg(
                Arg::new("v")
                    .short('v')
                    .multiple_occurrences(true)
                    .help("Sets the level of verbosity"),
            )
            .arg(
                Arg::new("video_src")
                    .long("video-src")
                    .default_value("shmsrc")
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
            .arg(Arg::new("h264_level")
                .long("h264_level")
                .default_value("4")
                .takes_value(true)
                .help("Set h264 decoder profile/level https://en.wikipedia.org/wiki/Advanced_Video_Coding#Levels")
            )            
            .arg(
                Arg::new("shm_src_socket")
                    .long("shm-src-socket")
                    .takes_value(true)
                    .help("shmsink socket path: https://gstreamer.freedesktop.org/documentation/shm/shmsink.html?gi-language=c")
            )
            .arg(
                Arg::new("shm_sink_socket")
                    .long("shm-sink-socket")
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
                Arg::new("shm_sync")
                .long("shm-sync")
                .takes_value(true)
                .default_value("false")
                .help("Set sync property on shmsink https://gstreamer.freedesktop.org/documentation/base/gstbasesink.html?gi-language=c")
            );

        app
    }

    fn on_sigint(&self) -> () {
        warn!("SIGINT received, removing {}", self.shm_sink_socket);
        fs::remove_file(&self.shm_sink_socket)
            .unwrap_or_else(|_| error!("Failed to delete file {}", &self.shm_sink_socket));
        process::exit(0)
    }

    fn build_pipeline(&self) -> Result<gst::Pipeline> {
        // initialize pipeline
        let pipeline = gst::Pipeline::new(Some("h264-rtp-payload"));

        // make input src element
        let src = gst::ElementFactory::make(&self.video_src.to_string(), Some("video_src"))?;

        // set socket-path for shmsrc
        if self.video_src == SrcOption::Shmsrc {
            src.set_property_from_str("socket-path", &self.shm_src_socket);
            src.set_property("is-live", true);
        }

        match self.video_src {
            SrcOption::Shmsrc => {
                src.set_property_from_str("socket-path", &self.shm_src_socket);
                src.set_property("is-live", true);
            },
            SrcOption::Videotestsrc => {
                src.set_property("is-live", true);

            },
            SrcOption::Libcamerasrc => ()
        };

        // set input caps
        let incapsfilter = gst::ElementFactory::make("capsfilter", Some("incapsfilter"))?;
        let incaps = gst::Caps::builder("video/x-raw")
            .field("width", &self.video_width)
            .field("height", &self.video_height)
            .field("framerate", gst::Fraction::new(0, 1))
            .field("format", "RGB")

            .build();
        incapsfilter.set_property("caps", incaps);

        // fallback to videoconvert element if v4l2convert is unavailable
        let converter = gst::ElementFactory::make("v4l2convert", None);
        let converter = match converter {
            Ok(r) => Ok(r),
            Err(e) => {
                warn!("Falling back to videoconvert element. error={:?}", e);
                gst::ElementFactory::make("videoconvert", None)
            }
        }?;


        // encode h264 video
        // fallback to x264enc if v4h264enc is unavailable
        let encoder = gst::ElementFactory::make("v4l2h264enc", None);
        let encoder = match encoder {
            Ok(e) => {
                // set v4l2h264 properties
                e.set_property_from_str("extra-controls", "controls,repeat_sequence_header=1");
                Ok(e)
            }
            Err(e) => {
                warn!("Falling back to x264enc element. error={:?}", e);
                gst::ElementFactory::make("x264enc", None)
            }
        }?;

        // set h264 encoder caps
        let encapsfilter = gst::ElementFactory::make("capsfilter", Some("encapsfilter"))?;
        let encaps = gst::Caps::builder("video/x-h264")
            .field("width", &self.video_width)
            .field("height", &self.video_height)
            .field("level", &self.h264_level)
            .build();
        encapsfilter.set_property("caps", encaps);

        // parse to rtp payload
        // let payloader = gst::ElementFactory::make("rtph264pay", None)?;
        // payloader.set_property_from_str("config-interval", "1");
        // payloader.set_property_from_str("aggregate-mode", "zero-latency");
        // payloader.set_property_from_str("pt", "96");
        


        // make shmsink element
        let shmsink = gst::ElementFactory::make("shmsink", None)?;
        shmsink.set_property_from_str("socket-path", &self.shm_sink_socket);
        shmsink.set_property("wait-for-connection", &self.shm_wait_for_connection);
        shmsink.set_property("shm-size", &self.shm_size);
        shmsink.set_property("sync", &self.shm_sync);

        let elements = [
            &src,
            &incapsfilter,
            &converter,
            &encoder,
            &encapsfilter,
            // &payloader,
            &shmsink
        ];


        pipeline.add_many(&elements)?;

        gst::Element::link_many(&elements)?;

    
        Ok(pipeline)
    }
}


impl Default for H264EncoderPipeline {
    fn default() -> Self {
        let paths = PrintNannyConfig::new().expect("Failed to initialize PrintNannyConfig").paths;
        return Self {
            h264_level: "4".into(),
            shm_size: 134217728, // 128MB
            shm_src_socket: paths.video_socket().display().to_string(),
            shm_sink_socket: paths.h264_rtp_payload_socket().display().to_string(),
            shm_wait_for_connection: true,
            shm_sync: true,
            video_height: 480,
            video_width: 640,
            video_src: SrcOption::Libcamerasrc,
        };
    }
}

impl From<&ArgMatches> for H264EncoderPipeline{
    fn from(args: &ArgMatches) -> Self {
        let defaults = H264EncoderPipeline::default();

        let video_height: i32 = args
            .value_of_t("video_height")
            .unwrap_or_else(|_| defaults.video_height);

        let video_width: i32 = args
            .value_of_t("video_width")
            .unwrap_or_else(|_| defaults.video_width);


        let video_src: &SrcOption = args
            .get_one::<SrcOption>("video_src")
            .unwrap_or_else(|| &defaults.video_src);

        let shm_size:u32 = args.value_of_t("shm_size").unwrap_or_else(|_| defaults.shm_size);


        let shm_wait_for_connection: bool = args
            .value_of_t("shm_wait_for_connection")
            .unwrap_or_else(|_| defaults.shm_wait_for_connection);


        let shm_src_socket: String = args
            .value_of_t("shm_src_socket")
            .unwrap_or_else(|_| defaults.shm_src_socket);

        let shm_sink_socket: String = args.value_of_t("shm_sink_socket").unwrap_or_else(|_| defaults.shm_sink_socket);

        let shm_sync: bool = args.is_present("shm_sync");

        let h264_level: String = args.value_of_t("h264_level").expect("--h264-level is required");


        Self {
            h264_level,
            shm_size,
            shm_src_socket,
            shm_sink_socket,
            shm_sync,
            shm_wait_for_connection,
            video_height,
            video_width,
            video_src: *video_src,
        }
    }
}
