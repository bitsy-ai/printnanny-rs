use anyhow::Result;
use env_logger::Builder;
use gst::prelude::*;

use log::warn;
use log::LevelFilter;
use printnanny_gst::h264_video::VideoSocketPipeline;
use printnanny_gst::pipeline::GstPipeline;

use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    // include git sha in version, which requires passing a boxed string to clap's .version() builder
    // parse args
    let cmd = VideoSocketPipeline::clap_command();
    let app_m = cmd.get_matches();
    let app = VideoSocketPipeline::from(&app_m);

    let pipeline = app.build_pipeline()?;
    // Need to move a new reference into the closure.
    // !!ATTENTION!!:
    // It might seem appealing to use pipeline.clone() here, because that greatly
    // simplifies the code within the callback. What this actually does, however, is creating
    // a memory leak. The clone of a pipeline is a new strong reference on the pipeline.
    // Storing this strong reference of the pipeline within the callback (we are moving it in!),
    // which is in turn stored in another strong reference on the pipeline is creating a
    // reference cycle.
    // DO NOT USE pipeline.clone() TO USE THE PIPELINE WITHIN A CALLBACK
    let pipeline_weak = pipeline.downgrade();

    tokio::spawn(async move {
        // Process each socket concurrently.
        process(socket).await
    });

    let handler = app.clone();
    ctrlc::set_handler(move || {
        warn!("Received Ctrl+C! Cleaning up app {:?}", &handler);
        handler.on_sigint();
    })?;

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
    app.run()?;
    Ok(())
}
