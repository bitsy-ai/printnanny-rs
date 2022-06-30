#[macro_use]
extern crate clap;

use anyhow::{bail, Result};
use clap::{Arg, ArgMatches, Command};
use env_logger::Builder;
use git_version::git_version;
use gstreamer::prelude::*;
use log::LevelFilter;

use printnanny_gst::error::ErrorMessage;
use printnanny_gst::options::{InputOption, VideoEncodingOption, VideoParameter};

pub struct App<'a> {
    video: VideoParameter,
    input: InputOption,
    tflite: bool,
    height: i32,
    width: i32,
    required_plugins: Vec<&'a str>,
}

impl App<'_> {
    pub fn new(args: &ArgMatches) -> Result<Self> {
        let mut required_plugins = vec!["videoconvert", "videoscale", "udp", "rtp"];
        // input src requirement
        let input = args.value_of_t("input")?;
        let mut input_reqs = match input {
            InputOption::Libcamerasrc => vec!["libcamerasrc"],
            InputOption::Videotestsrc => vec!["videotestsrc"],
        };
        required_plugins.append(&mut input_reqs);
        // encode in software vs hardware-accelerated
        let encoder_opt: VideoEncodingOption = args.value_of_t("encoder")?;
        let video: VideoParameter = encoder_opt.into();
        let mut encoder_reqs = video.requirements.split(' ').collect::<Vec<&str>>();
        required_plugins.append(&mut encoder_reqs);

        // tensorflow and nnstreamer requirements
        let tflite = args.is_present("tflite");
        match tflite {
            true => {
                let mut tf_reqs = vec![
                    "tensor_converter",
                    "tensor_transform",
                    "tensor_filter",
                    "tensor_decoder",
                ];
                required_plugins.append(&mut tf_reqs)
            }
            false => (),
        };

        let height: i32 = args.value_of_t("height").unwrap_or(480);
        let width: i32 = args.value_of_t("width").unwrap_or(480);
        Ok(Self {
            video,
            input,
            tflite,
            required_plugins,
            height,
            width,
        })
    }

    pub fn check_plugins(&self) -> Result<()> {
        let registry = gstreamer::Registry::get();
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
    fn build_simple_pipeline(&self) -> Result<gstreamer::Pipeline> {
        let p = format!(
            "{} \
            ! capsfilter caps=video/x-raw,width={},height={},framerate=0/1
            ! {} 
            ! {}
            ! testsink ",
            &self.input, &self.width, &self.height, &self.video.encoder, &self.video.payloader
        );
        let pipeline = gstreamer::parse_launch(&p)?;
        Ok(pipeline
            .downcast::<gstreamer::Pipeline>()
            .expect("Invalid gstreamer pipeline"))
    }

    fn build_tflite_pipeline(&self) -> Result<gstreamer::Pipeline> {
        let p = format!(
            "{} \
            ! capsfilter caps=video/x-raw,format=RGB,width={},height={},framerate=0/1
            ! {} 
            ! {}
            ! testsink ",
            &self.input, &self.width, &self.height, &self.video.encoder, &self.video.payloader
        );
        let pipeline = gstreamer::parse_launch(&p)?;
        Ok(pipeline
            .downcast::<gstreamer::Pipeline>()
            .expect("Invalid gstreamer pipeline"))
    }

    pub fn pipeline(&self) -> Result<gstreamer::Pipeline> {
        let p = match &self.tflite {
            true => self.build_tflite_pipeline(),
            false => self.build_simple_pipeline(),
        }?;
        Ok(p)
    }

    pub fn run(&self) -> Result<()> {
        let pipeline = self.pipeline()?;
        pipeline.set_state(gstreamer::State::Playing)?;

        // Create a stream for handling the GStreamer message asynchronously
        let bus = pipeline
            .bus()
            .expect("Pipeline without bus. Shouldn't happen!");
        let send_gst_msg_rx = bus.stream();
        for msg in bus.iter_timed(gstreamer::ClockTime::NONE) {
            use gstreamer::MessageView;
            match msg.view() {
                MessageView::Eos(..) => break,
                MessageView::Error(err) => {
                    pipeline.set_state(gstreamer::State::Null)?;
                    return Err(ErrorMessage {
                        src: msg
                            .src()
                            .map(|s| String::from(s.path_string()))
                            .unwrap_or_else(|| String::from("None")),
                        error: err.error().to_string(),
                        debug: err.debug(),
                    }
                    .into());
                }
                _ => (),
            }
        }
        pipeline.set_state(gstreamer::State::Null)?;
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
        .arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .help("Sets the level of verbosity"),
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
            Arg::new("input")
                .short('i')
                .long("input")
                .required(true)
                .takes_value(true)
                .possible_values(InputOption::possible_values())
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
        .arg(
            Arg::new("tflite")
                .required(false)
                .takes_value(false)
                .help("Run TensorFlow lite model on output"),
        );

    let app_m = app.get_matches();
    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'printnanny v v v' or 'printnanny vvv' vs 'printnanny v'
    let verbosity = app_m.occurrences_of("v");
    let mut builder = Builder::new();
    match verbosity {
        0 => builder.filter_level(LevelFilter::Warn).init(),
        1 => builder.filter_level(LevelFilter::Info).init(),
        2 => builder.filter_level(LevelFilter::Debug).init(),
        _ => builder.filter_level(LevelFilter::Trace).init(),
    };

    // Initialize GStreamer first
    gstreamer::init()?;
    // Check required_plugins plugins are installed
    let app = App::new(&app_m)?;

    app.check_plugins()?;
    app.run()?;

    Ok(())
}
