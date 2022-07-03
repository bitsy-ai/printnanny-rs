#[macro_use]
extern crate clap;

use anyhow::Result;
use clap::{Arg, Command};
use env_logger::Builder;
use git_version::git_version;
use log::LevelFilter;

use printnanny_gst::app::App;
use printnanny_gst::options::{AppModeOption, SinkOption, SrcOption, VideoEncodingOption};

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
            Arg::new("app")
                .default_value("rtp-video")
                .possible_values(AppModeOption::possible_values())
                .help("Application mode to run"),
        )
        .arg(
            Arg::new("video_height")
                .long("video_height")
                .default_value("480")
                .takes_value(true)
                .help("Input video height"),
        )
        .arg(
            Arg::new("video_width")
                .long("video_width")
                .default_value("640")
                .takes_value(true)
                .help("Input video width"),
        )
        .arg(
            Arg::new("src")
                .long("src")
                .required(true)
                .default_value("libcamerasrc")
                .takes_value(true)
                .possible_values(SrcOption::possible_values())
                .help("Pipeline source element"),
        )
        .arg(
            Arg::new("encoder")
                .short('e')
                .long("encoder")
                .required(true)
                .takes_value(true)
                .possible_values(VideoEncodingOption::possible_values())
                .help("Video encoding mdoe"),
        )
        .arg(
            Arg::new("sink")
                .long("sink")
                .required(true)
                .takes_value(true)
                .default_value("udpsink")
                .possible_values(SinkOption::possible_values())
                .help("Gstreamer sink"),
        )
        .arg(
            Arg::new("host")
                .long("host")
                .default_value("localhost")
                .takes_value(true)
                .required_if("sink", "udpsink")
                .help("udpsink host value"),
        )
        .arg(
            Arg::new("port_video")
                .long("port-video")
                .default_value("5104")
                .takes_value(true)
                .required_if("sink", "udpsink")
                .help("udpsink port value (original video stream)"),
        )
        .arg(
            Arg::new("port_overlay")
                .long("port-overlay")
                .default_value("5106")
                .takes_value(true)
                .required_if("sink", "udpsink")
                .help("udpsink port value (inference video overlay)"),
        )
        .arg(
            Arg::new("port_data")
                .long("port-data")
                .default_value("5107")
                .takes_value(true)
                .required_if("sink", "udpsink")
                .help("udpsink port value (inference tensor data)"),
        )
        .arg(
            Arg::new("tflite_model")
                .long("tflite-model")
                .default_value("/usr/share/printnanny/model/model.tflite")
                .takes_value(true)
                .required_if_eq_any(&[
                    ("app", "rtp-tflite-overlay"),
                    ("app", "rtp-tflite-composite"),
                ])
                .help("Path to model.tflite file"),
        )
        .arg(
            Arg::new("tflite_labels")
                .long("tflite-labels")
                .default_value("/usr/share/printnanny/model/dict.txt")
                .takes_value(true)
                .required_if_eq_any(&[
                    ("app", "rtp-tflite-overlay"),
                    ("app", "rtp-tflite-composite"),
                ])
                .help("Path to tflite labels file"),
        )
        .arg(
            Arg::new("tensor_height")
                .long("tensor-height")
                .default_value("320")
                .takes_value(true)
                .required_if_eq_any(&[
                    ("app", "rtp-tflite-overlay"),
                    ("app", "rtp-tflite-composite"),
                ])
                .help("Height of input tensor"),
        )
        .arg(
            Arg::new("tensor_width")
                .long("tensor-width")
                .default_value("320")
                .takes_value(true)
                .required_if_eq_any(&[
                    ("app", "rtp-tflite-overlay"),
                    ("app", "rtp-tflite-composite"),
                ])
                .help("Width of input tensor"),
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
    let app = App::new(&app_m)?;
    app.run()?;
    Ok(())
}
