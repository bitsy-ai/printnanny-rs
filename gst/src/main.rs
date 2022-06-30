#[macro_use]
extern crate clap;

use anyhow::{bail, Result};
use clap::{Arg, Command};
use env_logger::Builder;
use git_version::git_version;
use gstreamer::prelude::*;
use log::LevelFilter;

// Check if all GStreamer plugins we require are available
fn check_plugins(args: &clap::ArgMatches) -> Result<(), anyhow::Error> {
    let mut required = vec![
        "libcamerasrc",
        "videoconvert",
        "videoscale",
        "v4l2h264enc",
        "rtph264pay",
        "udpsink",
    ];

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
