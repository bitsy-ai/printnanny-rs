use anyhow::Result;
use gst::prelude::*;
use log::{debug, error, info};
use std::process;

// Basic gstreamer pipeline, wrapped in Clap command
pub trait GstPipeline {
    fn clap_command() -> clap::Command<'static>;
    fn build_pipeline(&self) -> Result<gst::Pipeline>;

    fn on_sigint(&self) -> () {
        info!("SIGINT received");
        process::exit(0);
    }

    fn run(&self, pipeline: gst::Pipeline) -> Result<()> {
        gst::init()?;
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
                MessageView::StateChanged(state_changed) => {
                    info!(
                        "Setting pipeline {:?} state to {:?}",
                        pipeline, &state_changed
                    );
                    // Generate a dot graph of the pipeline to GST_DEBUG_DUMP_DOT_DIR if defined

                    if state_changed.src().map(|s| s == pipeline).unwrap_or(false) {
                        pipeline.debug_to_dot_file(
                            gst::DebugGraphDetails::VERBOSE,
                            format!(
                                "{}-{:?}-{:?}",
                                pipeline.name(),
                                &state_changed.old(),
                                &state_changed.current()
                            ),
                        );
                    }
                }
                _ => debug!("No handler configured for msg {:?}", msg),
            }
        }
        info!("Setting pipeline {:?} state to Null", pipeline);
        pipeline
            .set_state(gst::State::Null)
            .expect("Unable to set the pipeline to the `Null` state");

        Ok(())
    }
}
