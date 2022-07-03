use anyhow::{bail, Result};
use clap::ArgMatches;
use gst::prelude::*;
use log::{error, info};

use printnanny_gst::error::MissingElement;
use printnanny_gst::options::{SinkOption, SrcOption, VideoEncodingOption, VideoParameter};

#[derive(Debug)]
pub struct BroadcastRtpVideo {
    pub host: String,
    pub video_port: i32,
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

pub struct App<'a> {
    pub video: VideoParameter,
    height: i32,
    width: i32,
    required_plugins: Vec<&'a str>,
    variant: AppVariant,
    encoder: VideoEncodingOption,
    pub src: SrcOption,
    pub sink: SinkOption,
}

impl App<'_> {
    pub fn new(args: &ArgMatches, sub_args: &ArgMatches, subcommand: &str) -> Result<Self> {
        let mut required_plugins = vec!["videoconvert", "videoscale"];
        // input src requirement
        let src: SrcOption = args.value_of_t("src")?;
        let sink = sub_args.value_of_t("sink").unwrap();
        let host = sub_args.value_of("host").unwrap().into();

        let mut input_reqs = match &src {
            SrcOption::Libcamerasrc => vec!["libcamera"],
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
                let video_port: i32 = sub_args.value_of_t("video_port").unwrap();
                let subapp = BroadcastRtpVideo { host, video_port };
                AppVariant::BroadcastRtpVideo(subapp)
            }
            "broadcast-rtp-tflite" => {
                // append rtp broadcast and tflite requirements
                let mut reqs = vec!["nnstreamer", "rtp", "udp"];
                required_plugins.append(&mut reqs);
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
                    tensor_width,
                };
                AppVariant::BroadcastRtpTfliteOverlay(subapp)
            }
            _ => bail!("Received unknown subcommand {}", subcommand),
        };

        let height: i32 = args.value_of_t("height").unwrap_or(480);
        let width: i32 = args.value_of_t("width").unwrap_or(480);

        Ok(Self {
            src,
            sink,
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
    // build a video pipeline, optionally linked from tee element
    fn build_video_pipeline(
        &self,
        pipeline: &gst::Pipeline,
        tee: Option<&gst::Element>,
    ) -> Result<()> {
        let src = gst::ElementFactory::make(&self.src.to_string(), None)?;
        let sink = gst::ElementFactory::make(&self.sink.to_string(), None)?;

        let queue =
            gst::ElementFactory::make("queue", None).map_err(|_| MissingElement("queue"))?;
        let videoconvert = gst::ElementFactory::make("videoconvert", None)
            .map_err(|_| MissingElement("videoconvert"))?;

        // set properties on src
        match &self.src {
            SrcOption::Videotestsrc => src.set_property("is-live", true),
            _ => (),
        };
        // set host / port on sink
        let (host, video_port) = match &self.variant {
            AppVariant::BroadcastRtpVideo(app) => (&app.host, &app.video_port),
            AppVariant::BroadcastRtpTfliteOverlay(app) => (&app.host, &app.video_port),
            AppVariant::BroadcastRtpTfliteComposite(app) => (&app.host, &app.video_port),
        };

        match &self.sink {
            SinkOption::Fakesink => (),
            SinkOption::Udpsink => {
                sink.set_property("host", &host);
                sink.set_property("port", &video_port);
            }
        };

        let incapsfilter = gst::ElementFactory::make("capsfilter", None)
            .map_err(|_| MissingElement("capsfilter"))?;
        let incaps = gst::Caps::builder("video/x-raw")
            .field("width", &self.width)
            .field("height", &self.height)
            .build();
        incapsfilter.set_property("caps", incaps);
        let encoder = match &self.encoder {
            VideoEncodingOption::H264Software => {
                let e = gst::ElementFactory::make("x264enc", None)
                    .map_err(|_| MissingElement("x264enc"))?;
                e.set_property_from_str("tune", "zerolatency");
                e
            }
            VideoEncodingOption::H264Hardware => {
                let e = gst::ElementFactory::make("v4l2h264enc", None)
                    .map_err(|_| MissingElement("v4l2h264enc"))?;
                e.set_property_from_str("extra-controls", "controls,repeat_sequence_header=1");
                e
            }
        };
        let payloader = gst::ElementFactory::make("rtph264pay", None)
            .map_err(|_| MissingElement("rtph264pay"))?;
        payloader.set_property_from_str("aggregate-mode", "zero-latency");

        let h264capsfilter = gst::ElementFactory::make("capsfilter", None)
            .map_err(|_| MissingElement("capsfilter"))?;
        let h264caps = gst::Caps::builder("video/x-h264")
            .field("level", "4")
            .build();
        h264capsfilter.set_property("caps", h264caps);
        pipeline.add_many(&[
            &src,
            &sink,
            &incapsfilter,
            &queue,
            &videoconvert,
            &encoder,
            &h264capsfilter,
            &payloader,
        ])?;
        match tee {
            Some(t) => gst::Element::link_many(&[
                t,
                &src,
                &incapsfilter,
                &queue,
                &videoconvert,
                &encoder,
                &h264capsfilter,
                &payloader,
                &sink,
            ])?,
            None => gst::Element::link_many(&[
                &src,
                &incapsfilter,
                &queue,
                &videoconvert,
                &encoder,
                &h264capsfilter,
                &payloader,
                &sink,
            ])?,
        };
        Ok(())
    }

    // build a tflite pipeline branch, intended for use with tee element
    fn build_tflite_pipeline(
        &self,
        pipeline: &gst::Pipeline,
        tee: Option<&gst::Element>,
    ) -> Result<()> {
        let queue =
            gst::ElementFactory::make("queue", None).map_err(|_| MissingElement("queue"))?;
        queue.set_property_from_str("leaky", "2");
        queue.set_property_from_str("max-size-buffers", "2");

        let pre_videoconvert = gst::ElementFactory::make("videoconvert", None)
            .map_err(|_| MissingElement("videoconvert"))?;

        let videoscale = gst::ElementFactory::make("videoscale", None)
            .map_err(|_| MissingElement("videoscale"))?;
        let pre_capsfilter = gst::ElementFactory::make("capsfilter", None)
            .map_err(|_| MissingElement("capsfilter"))?;

        let (tensor_width, tensor_height, tflite_model, tflite_labels, host, overlay_port) =
            match &self.variant {
                AppVariant::BroadcastRtpTfliteOverlay(app) => (
                    &app.tensor_width,
                    &app.tensor_height,
                    &app.tflite_model,
                    &app.tflite_labels,
                    &app.host,
                    &app.overlay_port,
                ),
                _ => unimplemented!(
                    "build_tflite_pipeline is not implemented for {:?}",
                    self.variant
                ),
            };
        let precaps = gst::Caps::builder("video/x-raw")
            .field("width", &tensor_width)
            .field("height", &tensor_height)
            .build();
        pre_capsfilter.set_property("caps", precaps);

        let tensor_converter = gst::ElementFactory::make("tensor_converter", None)
            .map_err(|_| MissingElement("tensor_converter"))?;

        let tensor_transform = gst::ElementFactory::make("tensor_transform", None)
            .map_err(|_| MissingElement("tensor_transform"))?;
        tensor_transform.set_property_from_str("mode", "arithmetic");
        tensor_transform.set_property_from_str("option", "typecast:uint8,add:0,div:1");

        let predict_tensor_filter = gst::ElementFactory::make("tensor_filter", None)
            .map_err(|_| MissingElement("tensor_filter"))?;
        predict_tensor_filter.set_property("framework", "tensorflow2-lite");
        predict_tensor_filter.set_property("model", tflite_model);

        let tensor_decoder = gst::ElementFactory::make("tensor_decoder", None)
            .map_err(|_| MissingElement("tensor_decoder"))?;
        tensor_decoder.set_property_from_str("mode", "bounding_boxes");
        tensor_decoder.set_property_from_str("option1", "mobilenet-ssd-postprocess");
        tensor_decoder.set_property_from_str("option2", tflite_labels);
        tensor_decoder.set_property_from_str("option3", "0:1:2:3,66");
        tensor_decoder.set_property_from_str("option4", &format!("{}:{}", self.width, self.height));
        tensor_decoder
            .set_property_from_str("option5", &format!("{}:{}", tensor_width, tensor_height));

        let post_videoconvert = gst::ElementFactory::make("videoconvert", None)
            .map_err(|_| MissingElement("videoconvert"))?;

        let post_capsfilter = gst::ElementFactory::make("capsfilter", None)
            .map_err(|_| MissingElement("capsfilter"))?;
        let post_caps = gst::Caps::builder("video/x-h264")
            .field("width", self.width)
            .field("height", self.height)
            .field("level", "4")
            .build();
        post_capsfilter.set_property("caps", post_caps);

        let post_videoenc = match &self.encoder {
            VideoEncodingOption::H264Software => {
                let e = gst::ElementFactory::make("x264enc", None)
                    .map_err(|_| MissingElement("x264enc"))?;
                e.set_property_from_str("tune", "zerolatency");
                e
            }
            VideoEncodingOption::H264Hardware => {
                let e = gst::ElementFactory::make("v4l2h264enc", None)
                    .map_err(|_| MissingElement("v4l2h264enc"))?;
                e.set_property_from_str("extra-controls", "controls,repeat_sequence_header=1");
                e
            }
        };
        let payloader = gst::ElementFactory::make("rtph264pay", None)
            .map_err(|_| MissingElement("rtph264pay"))?;
        payloader.set_property_from_str("aggregate-mode", "zero-latency");

        let sink = gst::ElementFactory::make(&self.sink.to_string(), None)?;
        match &self.sink {
            SinkOption::Fakesink => (),
            SinkOption::Udpsink => {
                sink.set_property("host", &host);
                sink.set_property("port", &overlay_port);
            }
        };

        pipeline.add_many(&[
            &queue,
            &pre_videoconvert,
            &post_videoconvert,
            &pre_capsfilter,
            &post_capsfilter,
            &videoscale,
            &tensor_transform,
            &tensor_converter,
            &predict_tensor_filter,
            &tensor_decoder,
            &post_videoenc,
            &payloader,
            &sink,
        ])?;

        match tee {
            Some(t) => gst::Element::link_many(&[
                t,
                &queue,
                &pre_videoconvert,
                &videoscale,
                &pre_capsfilter,
                &tensor_converter,
                &tensor_transform,
                &predict_tensor_filter,
                &tensor_decoder,
                &post_videoconvert,
                &post_videoenc,
                &post_capsfilter,
                &payloader,
                &sink,
            ])?,
            None => gst::Element::link_many(&[
                &queue,
                &pre_videoconvert,
                &videoscale,
                &pre_capsfilter,
                &tensor_converter,
                &tensor_transform,
                &predict_tensor_filter,
                &tensor_decoder,
                &post_videoconvert,
                &post_videoenc,
                &post_capsfilter,
                &payloader,
                &sink,
            ])?,
        };
        Ok(())
    }

    // build a tflite pipeline where inference results are rendered to overlay
    // overlay and original stream are broadcast to overlay_port and video_port
    fn build_broadcast_rtp_tflite_overlay_pipeline(&self, pipeline: &gst::Pipeline) -> Result<()> {
        let src = gst::ElementFactory::make(&self.src.to_string(), None)?;
        // set properties on src
        match &self.src {
            SrcOption::Videotestsrc => src.set_property("is-live", true),
            _ => (),
        };

        let tee = gst::ElementFactory::make("tee", None)?;
        pipeline.add_many(&[&src, &tee])?;
        gst::Element::link_many(&[&src, &tee])?;
        self.build_tflite_pipeline(pipeline, Some(&tee))?;
        self.build_video_pipeline(pipeline, Some(&tee))?;
        Ok(())
    }

    // build a tflite pipeline where inference results are composited to overlay
    fn build_broadcast_rtp_tflite_composite_pipeline(&self) -> Result<()> {
        unimplemented!("build_broadcast_rtp_tflite_composite_pipeline is not yet implemented")
    }

    pub fn build_pipeline(&self) -> Result<gst::Pipeline> {
        let pipeline = gst::Pipeline::new(None);
        match &self.variant {
            AppVariant::BroadcastRtpVideo(_) => self.build_video_pipeline(&pipeline, None),
            AppVariant::BroadcastRtpTfliteOverlay(_) => {
                self.build_broadcast_rtp_tflite_overlay_pipeline(&pipeline)
            }
            AppVariant::BroadcastRtpTfliteComposite(_) => {
                self.build_broadcast_rtp_tflite_composite_pipeline()
            }
        }?;
        Ok(pipeline)
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
