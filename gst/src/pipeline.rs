use anyhow::Result;
use gst::prelude::*;
use log::{error, info};

// Basic gstreamer pipeline, wrapped in Clap command
pub trait GstPipeline {
    fn clap_command() -> clap::Command<'static>;
    fn build_pipeline(&self) -> Result<gst::Pipeline>;
    fn run(&self) -> Result<()> {
        gst::init()?;
        let pipeline = self.build_pipeline()?;
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
                    // Generate a dot graph of the pipeline to GST_DEBUG_DUMP_DOT_DIR if defined

                    if state_changed.src().map(|s| s == pipeline).unwrap_or(false) {
                        pipeline.debug_to_dot_file(
                            gst::DebugGraphDetails::all(),
                            format!("{:?}-{:?}", &state_changed.old(), &state_changed.current()),
                        );
                    }
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
