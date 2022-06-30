#[macro_use]
extern crate clap;

use anyhow::{bail, Result};
use clap::{Arg, Command};
use env_logger::Builder;
use git_version::git_version;
use log::LevelFilter;

use printnanny_gst::options::{
    InputOption, VideoEncodingOption, VideoParameter, H264_HARDWARE, H264_SOFTWARE,
};

pub struct App {
    encoding: VideoParameter,
    input: InputOption,
    tflite: bool,
}

// Check if all GStreamer plugins we require are available
fn check_plugins(args: &clap::ArgMatches) -> Result<(), anyhow::Error> {
    let mut required = vec!["videoconvert", "videoscale", "udp", "rtp"];

    // input src requirement
    let mut input_reqs = match args.value_of_t("input")? {
        InputOption::Libcamerasrc => vec!["libcamerasrc"],
        InputOption::Videotestsrc => vec!["videotestsrc"],
    };
    required.append(&mut input_reqs);

    // encode in software vs hardware-accelerated
    let mut encoder_reqs = match args.value_of_t("encoder")? {
        VideoEncodingOption::H264Hardware => {
            H264_HARDWARE.requirements.split(' ').collect::<Vec<&str>>()
        }
        VideoEncodingOption::H264Software => {
            H264_SOFTWARE.requirements.split(' ').collect::<Vec<&str>>()
        }
    };
    required.append(&mut encoder_reqs);

    // tensorflow and nnstreamer requirements
    match args.is_present("tflite") {
        true => {
            let mut tf_reqs = vec![
                "tensor_converter",
                "tensor_transform",
                "tensor_filter",
                "tensor_decoder",
            ];
            required.append(&mut tf_reqs)
        }
        false => (),
    };

    let registry = gstreamer::Registry::get();
    let missing = required
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
            Arg::new("input")
                .short('i')
                .long("input")
                .required(true)
                .takes_value(true)
                .possible_values(InputOption::possible_values())
                .help("Run TensorFlow lite model on output"),
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
    // Check required plugins are installed
    check_plugins(&app_m)?;

    Ok(())
}
