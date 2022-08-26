use anyhow::Result;
use clap::{crate_authors, Arg, ArgMatches, Command, value_parser};
use gst::prelude::*;

use log::{error, info, warn};


use printnanny_services::config::PrintNannyConfig;

use super::options::SrcOption;
use super::pipeline::GstPipeline;


#[derive(Debug, Clone, PartialEq)]
pub struct RtpUdpPipeline {
    pub rtp_host: String,
    pub rtp_port: i32,
    pub video_height: i32,
    pub video_width: i32,
    pub video_src: SrcOption,
    pub h264_level: String,
    pub shm_src_socket: String,
}

impl GstPipeline for RtpUdpPipeline {
    fn clap_command() -> Command<'static> {
        let app_name = "rtp-udp.service";
        let app = Command::new(app_name)
            .author(crate_authors!())
            .about("Encode h264 video and RTP payloader, write to shared memory socket")
            .arg(
                Arg::new("v")
                    .short('v')
                    .multiple_occurrences(true)

                    .help("sets the level of verbosity")
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
                Arg::new("rtp_host")
                    .long("rtp-host")
                   .default_value("localhost")
                   .takes_value(true)
                   .help("RTP server hostname"),
            )
            .arg(
                Arg::new("rtp_port")
                    .long("rtp-port")
                   .default_value("5000")
                   .takes_value(true)
                   .help("RTP server port (UDP)"),
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
            );

        app
    }
    

    fn build_pipeline(&self) -> Result<gst::Pipeline> {
        info!("Initializing pipeline from settings {:?}", &self);
        // initialize pipeline
        let pipeline = gst::Pipeline::new(None);

        // make input src element
        let src = gst::ElementFactory::make(&self.video_src.to_string(), Some("video_src"))?;


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
        let encapsfilter = gst::ElementFactory::make("capsfilter", Some("encapsfilter"))?;
        let encaps = gst::Caps::builder("video/x-h264")
            .field("width", &self.video_width)
            .field("height", &self.video_height)
            .field("level", &self.h264_level)
            .build();
        encapsfilter.set_property("caps", encaps);

        let queue = gst::ElementFactory::make("queue2", None)?;

        let udpsink = gst::ElementFactory::make("udpsink", None)?;
        udpsink.set_property("host", &self.rtp_host);
        udpsink.set_property("port", &self.rtp_port);


        let elements = [
            &src,
            &encapsfilter,
            &queue,
            &udpsink
        ];

        pipeline.add_many(&elements)?;
        gst::Element::link_many(&elements)?;
        Ok(pipeline)

    }

}


impl Default for RtpUdpPipeline {
    fn default() -> Self {
        let paths = PrintNannyConfig::new().expect("Failed to initialize PrintNannyConfig").paths;
        return Self {
            h264_level: "4".into(),
            rtp_host: "localhost".into(),
            rtp_port: 5000,
            shm_src_socket: paths.h264_rtp_payload_socket().display().to_string(),
            video_height: 480,
            video_width: 640,
            video_src: SrcOption::Libcamerasrc,
        };
    }
}

impl From<&ArgMatches> for RtpUdpPipeline {
    fn from(args: &ArgMatches) -> Self {
        let defaults = RtpUdpPipeline::default();

        let video_height: i32 = args
            .value_of_t("video_height")
            .unwrap_or_else(|_| defaults.video_height);

        let video_width: i32 = args
            .value_of_t("video_width")
            .unwrap_or_else(|_| defaults.video_width);


        let video_src: &SrcOption = args
            .get_one::<SrcOption>("video_src")
            .unwrap_or_else(|| &defaults.video_src);


        let shm_src_socket: String = args
            .value_of_t("shm_src_socket")
            .unwrap_or_else(|_| defaults.shm_src_socket);


        let rtp_host = args.value_of_t("rtp_host").unwrap_or_else(|_| defaults.rtp_host);

        let rtp_port = args.value_of_t("rtp_port").unwrap_or_else(|_| defaults.rtp_port);



        let h264_level: String = args.value_of_t("h264_level").expect("--h264-level is required");


        Self {
            h264_level,
            rtp_host,
            rtp_port,
            shm_src_socket,
            video_height,
            video_width,
            video_src: *video_src,
        }
    }
}