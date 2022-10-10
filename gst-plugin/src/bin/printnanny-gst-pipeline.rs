// This example demonstrates the use of PrintNanny's detection algorithm against an .mp4 video source
// It operates the following pipeline:

// {videosrc} - {appsink}

// The application specifies what format it wants to handle. This format
// is applied by calling set_caps on the appsink. Now it's the audiotestsrc's
// task to provide this data format. If the element connected to the appsink's
// sink-pad were not able to provide what we ask them to, this would fail.
// This is the format we request:
// Audio / Signed 16bit / 1 channel / arbitrary sample rate

use clap::{crate_authors, crate_description, value_parser, Arg, ArgMatches, Command};
use gst::element_error;
use gst::glib;
use gst::prelude::*;

use byte_slice_cast::*;
use env_logger::Builder;

use std::i16;
use std::i32;

use anyhow::{Context, Error};
use git_version::git_version;
use log::{error, info, warn, LevelFilter};
use once_cell::sync::Lazy;

static CAT: Lazy<gst::DebugCategory> = Lazy::new(|| {
    gst::DebugCategory::new(
        "PrintNannyDemoVideo",
        gst::DebugColorFlags::empty(),
        Some("PritnNanny demo video pipeline"),
    )
});

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum VideoStreamSource {
    File,
    Device,
}

struct TfliteModel {
    label_file: String,
    model_file: String,
    nms_threshold: i32,
    tensor_batch_size: i32,
    tensor_channels: i32,
    tensor_height: i32,
    tensor_width: i32,
    tensor_queue_max_size_bytes: u32,
}

impl From<&ArgMatches> for TfliteModel {
    fn from(args: &ArgMatches) -> Self {
        let label_file = args
            .value_of("label_file")
            .expect("--label-file is required")
            .into();
        let model_file = args
            .value_of("model_file")
            .expect("--model-file is required")
            .into();
        let tensor_batch_size: i32 = args
            .value_of_t("tensor_batch_size")
            .context("--tensor-batch-size must be an integer")
            .unwrap();

        let tensor_height: i32 = args
            .value_of_t("tensor_height")
            .context("--tensor-height must be an integer")
            .unwrap();

        let tensor_width: i32 = args
            .value_of_t("tensor_width")
            .context("--tensor-width must be an integer")
            .unwrap();

        let tensor_channels: i32 = args
            .value_of_t("tensor_channels")
            .context("--tensor-channels must be an integer")
            .unwrap();

        let tensor_queue_max_size_bytes: u32 = args
            .value_of_t("tensor_queue_max_size_bytes")
            .context("--tensor-queue-max-size-bytes must be an integer")
            .unwrap();
        let nms_threshold: i32 = args
            .value_of_t("nms_threshold")
            .context("--nms-threshold must be an integer")
            .unwrap();
        Self {
            label_file,
            model_file,
            nms_threshold,
            tensor_batch_size,
            tensor_channels,
            tensor_height,
            tensor_queue_max_size_bytes,
            tensor_width,
        }
    }
}
struct PipelineApp {
    video_stream_src: VideoStreamSource,
    input_path: String,
    video_height: i32,
    video_width: i32,
    model: TfliteModel,
    udp_port: i32,
}

impl From<&ArgMatches> for PipelineApp {
    fn from(args: &ArgMatches) -> Self {
        let model = TfliteModel::from(args);

        let video_stream_src: &VideoStreamSource = args
            .get_one::<VideoStreamSource>("video_stream_src")
            .expect("--video-stream-src");

        let input_path = args
            .value_of("input_path")
            .expect("--video-file is required")
            .into();
        let video_height: i32 = args
            .value_of_t("video_height")
            .context("--video-height must be an integer")
            .unwrap();
        let video_width: i32 = args
            .value_of_t("video_width")
            .context("--video-width must be an integer")
            .unwrap();
        let udp_port: i32 = args.value_of_t("udp_port").context("--udp-port").unwrap();

        Self {
            model,
            input_path,
            video_height,
            video_width,
            video_stream_src: video_stream_src.clone(),
            udp_port,
        }
    }
}

impl PipelineApp {
    fn pipeline_src(&self) -> String {
        match self.video_stream_src {
            VideoStreamSource::File => format!(
                "filesrc location={input_path} do-timestamp=true",
                input_path = self.input_path
            ),
            VideoStreamSource::Device => format!("libcamerasrc"),
        }
    }

    pub fn create_pipeline(&self) -> Result<gst::Pipeline, Error> {
        gst::init()?;

        let pipeline_src = self.pipeline_src();
        let pipeline_str = format!(
            "{pipeline_src} \
            ! qtdemux name=demux \
            demux.video_0 ! decodebin \
            ! tee name=decoded_video_t \
            decoded_video_t. \
            ! queue name=decoded_video_tensor_q \
            ! videoscale \
            ! videoconvert \
            ! capsfilter caps=video/x-raw,width={tensor_width},height={tensor_height},format=RGB \
            ! tensor_converter
            ! tensor_transform mode=arithmetic option=typecast:uint8,add:0,div:1 \
            ! capsfilter caps=other/tensors,num_tensors=1,format=static \
            ! queue name=tensor_filter_q \
            ! tensor_filter framework=tensorflow2-lite model={model_file} \
            ! tee name=tensor_t 
            ! queue name=tensor_decoder_q \
            ! tensor_rate framerate={framerate}/1 throttle=true \
            ! tensor_decoder mode=bounding_boxes \
                option1=mobilenet-ssd-postprocess \
                option2={label_file} \
                option3=0:1:2:3,{nms_threshold} \
                option4={video_width}:{video_height} \
                option5={tensor_width}:{tensor_height} \
            ! queue name=compositor_q \
            ! compositor name=comp sink_0::zorder=2 sink_1::zorder=1 \
            ! timeoverlay \
            ! encodebin profile=\"video/x-h264,tune=zerolatency,profile=main\" \
            ! rtph264pay config-interval=1 aggregate-mode=zero-latency pt=96 \
            ! queue2 \
            ! udpsink port={udp_port} \
            decoded_video_t. ! queue name=videoscale_q \
            ! videoscale \
            ! capsfilter caps=video/x-raw,width={video_width},height={video_height} ! comp.sink_1 \
            tensor_t. ! queue name=custom_tensor_decoder_t ! tensor_decoder mode=custom-code option1=printnanny_bb_dataframe_decoder \
            ! dataframe_agg filter-threshold=0.5 output-type=json \
            ! queue2 \
            ! nats_sink \
            ",
            tensor_height = &self.model.tensor_height,
            tensor_width = &self.model.tensor_width,
            model_file = &self.model.model_file,
            label_file = &self.model.label_file,
            nms_threshold = &self.model.nms_threshold,
            video_width = &self.video_width,
            video_height = &self.video_height,
            framerate = 15,
            udp_port = &self.udp_port,
            pipeline_src = pipeline_src
        );

        let pipeline = gst::parse_launch(&pipeline_str)?;
        let pipeline = pipeline.dynamic_cast::<gst::Pipeline>().unwrap();

        Ok(pipeline)
    }
}

fn run(pipeline: gst::Pipeline) -> Result<(), Error> {
    pipeline.set_state(gst::State::Playing)?;

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                pipeline.set_state(gst::State::Null)?;
                gst::error!(CAT, "demo-video pipeline failed with error: {:?}", err);
            }
            MessageView::StateChanged(state_changed) => {
                gst::info!(
                    CAT,
                    "Setting pipeline {:?} state to {:?}",
                    pipeline,
                    &state_changed
                );
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
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;

    Ok(())
}

fn main() {
    let mut log_builder = Builder::new();

    let app_name = "printnanny-video-demo";
    const GIT_VERSION: &str = git_version!();

    let cmd = Command::new(app_name)
        .author(crate_authors!())
        .about(crate_description!())
        // show git sha in --version
        .version(GIT_VERSION)
        // set level of verbosity
        .arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .help("Sets the level of verbosity. Info: -v Debug: -vv Trace: -vvv"),
        )
        .arg(
            Arg::new("udp_port")
                .long("--udp-port")
                .takes_value(true)
                .default_value("20001")
                .help("Janus RTP stream port (UDP)"),
        )
        // --nms-threshold
        .arg(
            Arg::new("nms_threshold")
                .long("--nms-threshold")
                .takes_value(true)
                .default_value("50")
                .help("Non-max supression threshold"),
        )
        // --input-path
        .arg(
            Arg::new("input_path")
                .long("input-path")
                .required(true)
                .takes_value(true)
                .help("Path to video file or camera device"),
        )
        .arg(
            Arg::new("video_height")
                .long("video-height")
                .default_value("720")
                .takes_value(true)
                .help("Height of input video file"),
        )
        .arg(
            Arg::new("video_width")
                .long("video-width")
                .default_value("960")
                .takes_value(true)
                .help("Width of input video file"),
        )
        // --video-stream-src
        .arg(
            Arg::new("video_stream_src")
                .long("video-stream-src")
                .value_parser(value_parser!(VideoStreamSource))
                .takes_value(true),
        )
        // --tensor-queue-max-size-bytes
        .arg(
            Arg::new("tensor_queue_max_size_bytes")
                .long("tensor-queue-max-size-bytes")
                .takes_value(true)
                .default_value("33554432")
                .help("Max amount of data to hold in tensor queue, in bytes"),
        )
        // --tensor-batch-size
        .arg(
            Arg::new("tensor_batch_size")
                .long("tensor-batch-size")
                .takes_value(true)
                .default_value("1")
                .help("Batch size for tensor with shape: [Batch size, Height, Width, Channels]"),
        )
        // --tensor-height
        .arg(
            Arg::new("tensor_height")
                .long("tensor-height")
                .takes_value(true)
                .default_value("320")
                .help("Height value for tensor with shape: [Batch size, Height, Width, Channels]"),
        )
        .arg(
            Arg::new("tensor_width")
                .long("tensor-width")
                .takes_value(true)
                .default_value("320")
                .help("Width value for tensor with shape: [Batch size, Height, Width, Channels]"),
        )
        .arg(
            Arg::new("tensor_channels")
                .long("tensor-channels")
                .takes_value(true)
                .default_value("3")
                .help(
                    "Channels value for tensor with shape: [Batch size, Height, Width, Channels]",
                ),
        )
        .arg(
            Arg::new("model_file")
                .long("model-file")
                .takes_value(true)
                .required(true)
                .help("Path to .tflite model file"),
        )
        .arg(
            Arg::new("label_file")
                .long("label-file")
                .takes_value(true)
                .required(true)
                .help("Path to labels.txt file"),
        );
    let args = cmd.get_matches();
    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'printnanny -vvv' or 'printnanny -vv' vs 'printnanny -v'
    let verbosity = args.occurrences_of("v");
    match verbosity {
        0 => {
            log_builder.filter_level(LevelFilter::Warn).init();
            gst::debug_set_default_threshold(gst::DebugLevel::Warning);
        }
        1 => {
            log_builder.filter_level(LevelFilter::Info).init();
            gst::debug_set_default_threshold(gst::DebugLevel::Info);
        }
        2 => {
            log_builder.filter_level(LevelFilter::Debug).init();
            gst::debug_set_default_threshold(gst::DebugLevel::Debug);
        }
        _ => {
            gst::debug_set_default_threshold(gst::DebugLevel::Trace);
            log_builder.filter_level(LevelFilter::Trace).init()
        }
    };
    let app = PipelineApp::from(&args);
    match app.create_pipeline().and_then(run) {
        Ok(r) => r,
        Err(e) => error!("Error running pipeline: {:?}", e),
    }
}
