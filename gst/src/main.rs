#[macro_use]
extern crate clap;

use anyhow::{bail, Result, anyhow};
use clap::{Arg, ArgMatches, Command};
use env_logger::Builder;
use git_version::git_version;
use gst::prelude::*;
use log::{ error, info};
use log::LevelFilter;
use anyhow::Error;
use derive_more::{Display, Error};


use printnanny_gst::options::{SrcOption, VideoEncodingOption, VideoParameter, SinkOption};
use printnanny_gst::error::MissingElement;

#[derive(Debug)]
pub struct BroadcastRtpVideo {
    pub host: String,
    pub video_port: i32,
    pub src: SrcOption,
    pub sink: SinkOption
}

#[derive(Debug)]
pub struct BroadcastRtpVideoOverlay {
    pub host: String,
    pub video_port: i32,
    pub data_port: i32,
    pub overlay_port: i32,

    pub tensor_height: i32,
    pub tensor_width: i32,
    pub tflite_model: String,
    pub tflite_labels: String,
}

#[derive(Debug)]
pub enum AppVariant {
    // broadcast source video stream over 1 rtp port (light compute)
    BroadcastRtpVideo(BroadcastRtpVideo),
    // broadcast source video, model inference video, and model inference tensor over 3 rtp ports (medium compute)
    BroadcastRtpTfliteOverlay(BroadcastRtpVideoOverlay),
    // broadcast video composited from source / inference (heavy compute)
    BroadcastRtpTfliteComposite(BroadcastRtpVideoOverlay),
}

#[derive(Debug)]
pub struct App<'a> {
    debug: bool,
    video: VideoParameter,
    height: i32,
    width: i32,
    required_plugins: Vec<&'a str>,
    variant: AppVariant,
    encoder: VideoEncodingOption,
}

impl App<'_> {
    pub fn new(args: &ArgMatches, sub_args: &ArgMatches, subcommand: &str) -> Result<Self> {
        let debug = args.is_present("debug");
        let mut required_plugins = vec!["videoconvert", "videoscale"];
        // input src requirement
        let src: SrcOption = args.value_of_t("src")?;
        let mut input_reqs = match &src {
            SrcOption::Libcamerasrc => vec!["libcamerasrc"],
            SrcOption::Videotestsrc => vec!["videotestsrc"],
        };
        required_plugins.append(&mut input_reqs);
        // encode in software vs hardware-accelerated
        let encoder: VideoEncodingOption = args.value_of_t("encoder")?;
        let video: VideoParameter = encoder.into();
        let mut encoder_reqs = video.requirements.split(' ').collect::<Vec<&str>>();
        required_plugins.append(&mut encoder_reqs);

        // tensorflow and nnstreamer requirements
        let variant: AppVariant = match subcommand {
            "broadcast-rtp-video" => {
                // append rtp broadcast requirements
                let mut reqs = vec!["rtp", "udp"];
                required_plugins.append(&mut reqs);
                let host = sub_args.value_of("host").unwrap().into();
                let video_port: i32 = sub_args.value_of_t("video_port").unwrap();
                let sink = sub_args.value_of_t("sink").unwrap();
                let subapp = BroadcastRtpVideo { host, video_port, sink, src };
                AppVariant::BroadcastRtpVideo(subapp)
            }
            "broadcast-rtp-tflite" => {
                // append rtp broadcast and tflite requirements
                let mut reqs = vec![
                    "nnstreamer",
                    "rtp",
                    "udp",
                ];
                required_plugins.append(&mut reqs);
                let host = sub_args.value_of("host").unwrap().into();
                let video_port: i32 = sub_args.value_of_t("video_port").unwrap();
                let data_port: i32 = sub_args.value_of_t("data_port").unwrap();
                let overlay_port: i32 = sub_args.value_of_t("overlay_port").unwrap();

                let tflite_model = sub_args.value_of("tflite_model").unwrap().into();
                let tflite_labels = sub_args.value_of("tflite_labels").unwrap().into();
                let tensor_height: i32 = sub_args.value_of_t("tensor_height").unwrap();
                let tensor_width: i32 = sub_args.value_of_t("tensor_width").unwrap();

                let subapp = BroadcastRtpVideoOverlay {
                    host,
                    video_port,
                    data_port,
                    overlay_port,
                    tflite_labels,
                    tflite_model,
                    tensor_height,
                    tensor_width
                };
                AppVariant::BroadcastRtpTfliteOverlay(subapp)
            }
            _ => bail!("Received unknown subcommand {}", subcommand),
        };

        let height: i32 = args.value_of_t("height").unwrap_or(480);
        let width: i32 = args.value_of_t("width").unwrap_or(480);

        Ok(Self {
            debug,
            encoder,
            video,
            required_plugins,
            height,
            width,
            variant,
        })
    }

    pub fn check_plugins(&self) -> Result<()> {
        let registry = gst::Registry::get();
        let missing = self
            .required_plugins
            .iter()
            .filter(|n| registry.find_plugin(n).is_none())
            .cloned()
            .collect::<Vec<_>>();

        if !missing.is_empty() {
            bail!("Missing plugins: {:?}", missing);
        } else {
            Ok(())
        }
    }
    // build a video-only pipeline without tflite inference
    fn build_broadcast_rtp_video_pipeline(
        &self,
        app: &BroadcastRtpVideo,
    ) -> Result<gst::Pipeline> {

        let src = gst::ElementFactory::make(&app.src.to_string(), None)?;
        let sink = gst::ElementFactory::make(&app.sink.to_string(), None)?;

        let queue = gst::ElementFactory::make("queue", None).map_err(|_| MissingElement("queue"))?;
        let videoconvert = gst::ElementFactory::make("videoconvert", None).map_err(|_| MissingElement("videoconvert"))?;

        info!("Created from app: {:?} {:?}", self, app);
        
        // set properties on src
        match &app.src {
            SrcOption::Videotestsrc => src.set_property("is-live", true),
            _ => ()
        };
        // set host / port on sink
        match &app.sink {
            SinkOption::Fakesink => (),
            SinkOption::Udpsink => {
                sink.set_property("host", &app.host);
                sink.set_property("port", &app.video_port);
            }
        };

        let incapsfilter = gst::ElementFactory::make("capsfilter", None).map_err(|_| MissingElement("capsfilter"))?;
        let incaps = gst::Caps::builder("video/x-raw")
            .field("width", &self.width)
            .field("height", &self.height)
            .build();
        incapsfilter.set_property("caps", incaps);
 
        let encoder = match &self.encoder {
            VideoEncodingOption::H264Software => {
                let e = gst::ElementFactory::make("x264enc", None).map_err(|_| MissingElement("x264enc"))?;
                e.set_property_from_str("tune", "zerolatency");
                e
            }
            VideoEncodingOption::H264Hardware => {
                let e = gst::ElementFactory::make("v4l2h264enc", None).map_err(|_| MissingElement("v4l2h264enc"))?;
                e.set_property_from_str("extra-controls", "controls,repeat_sequence_header=1");
                e
            }
        };
        let payloader = gst::ElementFactory::make("rtph264pay", None).map_err(|_| MissingElement("rtph264pay"))?;
        payloader.set_property_from_str("aggregate-mode", "zero-latency");

        let h264capsfilter = gst::ElementFactory::make("capsfilter", None).map_err(|_| MissingElement("capsfilter"))?;
        let h264caps = gst::Caps::builder("video/x-h264")
            .field("level", "4")
            .build();
        h264capsfilter.set_property("caps", h264caps);

        let pipeline = gst::Pipeline::new(None);

        // pipeline.add_many(&[&src, &incapsfilter, &sink ])?;
        // src.link(&incapsfilter)?;
        // incapsfilter.link(&sink)?;

        pipeline.add_many(&[&src, &sink, &incapsfilter, &queue, &videoconvert, &encoder, &h264capsfilter, &payloader])?;
        src.link(&incapsfilter)?;
        incapsfilter.link(&queue)?;
        queue.link(&videoconvert)?;
        videoconvert.link(&encoder)?;
        encoder.link(&h264capsfilter)?;
        h264capsfilter.link(&payloader)?;
        payloader.link(&sink)?;
        Ok(pipeline)
    
    }

    // build a tflite pipeline where inference results are rendered to overlay
    // overlay and original stream are broadcast to overlay_port and video_port
    fn build_broadcast_rtp_tflite_overlay_pipeline(
        &self,
        app: &BroadcastRtpVideoOverlay,
    ) -> Result<gst::Pipeline> {
        unimplemented!("build_broadcast_rtp_tflite_overlay_pipeline")
        // let p = format!(
        //     "{input}
        //     ! capsfilter caps=video/x-raw,format=RGB,width={width},height={height},framerate=0/1
        //     ! tee name=t
        //         t.  ! queue leaky=2 max-size-buffers=2
        //             ! videoconvert
        //             ! videoscale ! video/x-raw,width={tensor_width},height={tensor_height}
        //             ! tensor_converter
        //             ! tensor_transform mode=arithmetic option=typecast:uint8,add:0,div:1
        //             ! other/tensors,num_tensors=1,format=static
        //             ! tensor_filter framework=tensorflow2-lite model={model}
        //             ! tensor_decoder mode=bounding_boxes option1=mobilenet-ssd-postprocess option2={labels} option3=0:1:2:3,66 option4={width}:{height} option5={tensor_height}:{tensor_width}
        //             ! videoconvert
        //             ! {encoder}
        //             ! 'video/x-h264,width=640,height=480,level=(string)4'
        //             ! {parser}
        //             ! {payloader}
        //             ! udpsink host={host} port={overlay_port}
        //         t.  ! queue leaky=2 max-size-buffers=2
        //             ! videoconvert
        //             ! {encoder}
        //             ! 'video/x-h264,width=640,height=480,level=(string)4'
        //             ! {parser}
        //             ! {payloader}
        //             ! udpsink host={host} port={video_port}",
        //     input = self.input,
        //     width = self.width,
        //     height = self.height,
        //     encoder = self.video.encoder,
        //     payloader = self.video.payloader,
        //     host = app.host,
        //     video_port = app.video_port,
        //     overlay_port = app.overlay_port,
        //     tensor_height = app.tensor_height,
        //     tensor_width = app.tensor_width,
        //     model = app.tflite_model,
        //     labels = app.tflite_labels,
        //     parser = self.video.parser
        // );
        // let pipeline = gst::parse_launch(&p)?;
        // Ok(pipeline
        //     .downcast::<gst::Pipeline>()
        //     .expect("Invalid gstreamer pipeline"))
    }

    // build a tflite pipeline where inference results are composited to overlay
    fn build_broadcast_rtp_tflite_composite_pipeline(
        &self,
        app: &BroadcastRtpVideoOverlay,
    ) -> Result<gst::Pipeline> {
        unimplemented!("build_broadcast_rtp_tflite_composite_pipeline is not yet implemented")
    }


    pub fn build_pipeline(&self) -> Result<gst::Pipeline> {
        match &self.variant {
            AppVariant::BroadcastRtpVideo(app) => self.build_broadcast_rtp_video_pipeline(app),
            AppVariant::BroadcastRtpTfliteOverlay(app) => {
                self.build_broadcast_rtp_tflite_overlay_pipeline(app)
            }
            AppVariant::BroadcastRtpTfliteComposite(app) => {
                self.build_broadcast_rtp_tflite_composite_pipeline(app)
            }
        }
    }

    pub fn play(&self) -> Result<()> {
        let pipeline = self.build_pipeline()?;
        info!("Setting pipeline {:?} state to Playing", pipeline);
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
    let app_name = "printnanny-gst";
    let app = Command::new(app_name)
        .author(crate_authors!())
        .about(crate_description!())
        .version(&version[..])
        .subcommand_required(true)
        // generic app args
        .arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::new("debug")
                .help("Run pipeline with debug src/sink"),
        )
        .arg(
            Arg::new("height")
                .long("height")
                .default_value("480")
                .takes_value(true)
                .help("Input resolution height"),
        )
        .arg(
            Arg::new("width")
                .long("width")
                .default_value("640")
                .takes_value(true)
                .help("Input resolution width"),
        )
        .arg(
            Arg::new("src")
                .long("src")
                .required(true)
                .takes_value(true)
                .possible_values(SrcOption::possible_values())
                .help(""),
        )
        .arg(
            Arg::new("encoder")
                .short('e')
                .long("encoder")
                .required(true)
                .takes_value(true)
                .possible_values(VideoEncodingOption::possible_values())
                .help("Run TensorFlow lite model on output"),
        )
        // tflite app args
        .subcommand(
            Command::new("broadcast-rtp-tflite")
                .author(crate_authors!())
                .about(
                "Run TensorFlow Lite inference over stream, broadcast encoded video stream and inference results over rtp",
            )
            .arg(
                Arg::new("host")
                    .long("host")
                    .default_value("localhost")
                    .takes_value(true)
                    .help("udpsink host value"),
            )
            .arg(
                Arg::new("video_port")
                    .long("video-port")
                    .default_value("5104")
                    .takes_value(true)
                    .help("udpsink port value (original video stream)"),
            )
            .arg(
                Arg::new("overlay_port")
                    .long("overlay-port")
                    .default_value("5106")
                    .takes_value(true)
                    .help("udpsink port value (inference video overlay)"),
            )
            .arg(
                Arg::new("data_port")
                    .long("data-port")
                    .default_value("5107")
                    .takes_value(true)
                    .help("udpsink port value (inference tensor data)"),
            )
            .arg(
                Arg::new("tflite_model")
                    .long("tflite-model")
                    .default_value("/usr/share/printnanny/model/model.tflite")
                    .takes_value(true)
                    .help("Path to model.tflite file"),
            )
            .arg(
                Arg::new("tflite_labels")
                    .long("tflite-labels")
                    .default_value("/usr/share/printnanny/model/dict.txt")
                    .takes_value(true)
                    .help("Path to tflite labels file"),
            )
            .arg(
                Arg::new("tensor_height")
                    .long("tensor-height")
                    .default_value("320")
                    .takes_value(true)
                    .help("Height of input tensor"),
            )
            .arg(
                Arg::new("tensor_width")
                    .long("tensor-width")
                    .default_value("320")
                    .takes_value(true)
                    .help("Width of input tensor"),
            )
        )
        // simple video app args
        .subcommand(
            Command::new("broadcast-rtp-video")
                .author(crate_authors!())
                .about("Encode video and broadcast over rtp")
            .arg(
                Arg::new("sink")
                    .long("sink")
                    .required(true)
                    .takes_value(true)
                    .possible_values(SinkOption::possible_values())
                    .help("Gstreamer sink"),
               )
            .arg(
                Arg::new("host")
                    .long("host")
                    .default_value("localhost")
                    .takes_value(true)
                    .required_if("sink", "udpsink" )
                    .help("udpsink host value"),
            )
            .arg(
                Arg::new("video_port")
                    .long("video-port")
                    .default_value("5104")
                    .takes_value(true)
                    .required_if("sink", "udpsink")
                    .help("udpsink port value"),
            )
            
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
        },
        1 => {
            builder.filter_level(LevelFilter::Info).init();
            gst::debug_set_default_threshold(gst::DebugLevel::Info);
        },
        2 => {
            builder.filter_level(LevelFilter::Debug).init();
            gst::debug_set_default_threshold(gst::DebugLevel::Debug);

        },
        _ => {
            gst::debug_set_default_threshold(gst::DebugLevel::Trace);
            builder.filter_level(LevelFilter::Trace).init()
        },
    };
    

    // Initialize GStreamer first
    gst::init()?;
    // Check required_plugins plugins are installed

    let (subcommand, sub_m) = app_m.subcommand().unwrap();
    let app = App::new(&app_m, &sub_m, &subcommand)?;

    app.check_plugins()?;
    app.play()?;

    Ok(())
}
