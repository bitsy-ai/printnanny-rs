use gst::prelude::*;
use gst::MessageView;

use polars::io::ipc::IpcStreamReader;
use polars::io::SerReader;
use polars::prelude::*;

use std::fs::File;
use std::path::PathBuf;

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        gst::init().unwrap();
        gstprintnanny::plugin_register_static().unwrap();
    });
}

// requires nats server to be running, ignore in CI but keep as development helper
#[ignore]
#[test]
fn test_nats_sink() {
    init();
    let base_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model_path: PathBuf = base_path.join("fixtures/model.tflite");
    let num_detections = 40;
    let expected_buffers = 16;

    let pipeline_str = format!(
        "videotestsrc num-buffers={expected_buffers} \
        ! capsfilter caps=video/x-raw,width={tensor_width},height={tensor_height},format=RGB \
        ! videoscale \
        ! videoconvert \
        ! tensor_converter \
        ! capsfilter caps=other/tensors,num_tensors=1,format=static \
        ! tensor_filter framework=tensorflow2-lite model={model_file} output=4:{num_detections}:1:1,{num_detections}:1:1:1,{num_detections}:1:1:1,1:1:1:1 outputname=detection_boxes,detection_classes,detection_scores,num_detections outputtype=float32,float32,float32,float32 \
        ! tensor_decoder mode=custom-code option1=printnanny_bb_dataframe_decoder \
        ! nats_sink",
        expected_buffers = expected_buffers,
        num_detections = num_detections,
        tensor_width = 320,
        tensor_height = 320,
        model_file = model_path.display()
    );

    let pipeline = gst::parse_launch(&pipeline_str).expect("Failed to construct pipeline");
    pipeline.set_state(gst::State::Playing).unwrap();
    let bus = pipeline.bus().unwrap();
    let mut events = vec![];

    loop {
        let msg = bus.iter_timed(gst::ClockTime::NONE).next().unwrap();

        match msg.view() {
            MessageView::Error(_) | MessageView::Eos(..) => {
                events.push(msg.clone());
                break;
            }
            // check stream related messages
            MessageView::StreamCollection(_) | MessageView::StreamsSelected(_) => {
                events.push(msg.clone())
            }
            _ => {}
        }
    }
    pipeline.set_state(gst::State::Null).unwrap();
}

#[test]
fn test_nnstreamer_callback() {
    init();
    let base_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model_path: PathBuf = base_path.join("fixtures/model.tflite");

    let num_detections = 40;

    let expected_buffers = 16;
    let pipeline = format!(
        "videotestsrc num-buffers={expected_buffers} \
        ! capsfilter caps=video/x-raw,width={tensor_width},height={tensor_height},format=RGB \
        ! videoscale \
        ! videoconvert \
        ! tensor_converter \
        ! capsfilter caps=other/tensors,num_tensors=1,format=static \
        ! tensor_filter framework=tensorflow2-lite model={model_file} output=4:{num_detections}:1:1,{num_detections}:1:1:1,{num_detections}:1:1:1,1:1:1:1 outputname=detection_boxes,detection_classes,detection_scores,num_detections outputtype=float32,float32,float32,float32 \
        ! tensor_decoder mode=custom-code option1=printnanny_bb_dataframe_decoder",
        expected_buffers = expected_buffers,
        num_detections = num_detections,
        tensor_width = 320,
        tensor_height = 320,
        model_file = model_path.display()
    );
    let mut h = gst_check::Harness::new_parse(&pipeline);
    let bus = gst::Bus::new();
    let element = h.element().unwrap();
    element.set_bus(Some(&bus));
    h.play();

    let mut num_buffers = 0;
    while let Some(buffer) = h.pull_until_eos().unwrap() {
        let cursor = buffer.as_cursor_readable();
        let df = IpcStreamReader::new(cursor)
            .finish()
            .expect("Failed to extract dataframe");

        // dataframe should have 6 columns and num_detections rows
        assert_eq!(df.shape(), (num_detections, 6));

        println!("Pulled dataframe from buffer {:?}", df);
        num_buffers += 1;
    }
    assert_eq!(num_buffers, expected_buffers);
}

// TODO: test flakes on:
// `Err` value: ComputeError(Borrowed("empty container given"))'
#[ignore]
#[test]
fn test_dataframe_filesink() {
    init();
    let base_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model_path: PathBuf = base_path.join("fixtures/model.tflite");
    let tmp_dir = base_path.join(".tmp");

    let dataframe_location = format!("{}/videotestsrc_%05d.ipc", tmp_dir.display());
    let num_detections = 40;
    let expected_buffers = 16;

    let pipeline_str = format!(
        "videotestsrc num-buffers={expected_buffers} \
        ! capsfilter caps=video/x-raw,width={tensor_width},height={tensor_height},format=RGB \
        ! videoscale \
        ! videoconvert \
        ! tensor_converter \
        ! capsfilter caps=other/tensors,num_tensors=1,format=static \
        ! tensor_filter framework=tensorflow2-lite model={model_file} output=4:{num_detections}:1:1,{num_detections}:1:1:1,{num_detections}:1:1:1,1:1:1:1 outputname=detection_boxes,detection_classes,detection_scores,num_detections outputtype=float32,float32,float32,float32 \
        ! tensor_decoder mode=custom-code option1=printnanny_bb_dataframe_decoder \
        ! dataframe_filesink location={dataframe_location}
        ",
        expected_buffers = expected_buffers,
        num_detections = num_detections,
        tensor_width = 320,
        tensor_height = 320,
        model_file = model_path.display()
    );

    let pipeline = gst::parse_launch(&pipeline_str).expect("Failed to construct pipeline");
    pipeline.set_state(gst::State::Playing).unwrap();
    let bus = pipeline.bus().unwrap();
    let mut events = vec![];

    loop {
        let msg = bus.iter_timed(gst::ClockTime::NONE).next().unwrap();

        match msg.view() {
            MessageView::Error(_) | MessageView::Eos(..) => {
                events.push(msg.clone());
                break;
            }
            // check stream related messages
            MessageView::StreamCollection(_) | MessageView::StreamsSelected(_) => {
                events.push(msg.clone())
            }
            _ => {}
        }
    }
    pipeline.set_state(gst::State::Null).unwrap();

    let pattern = format!("{}/videotestsrc*.ipc", tmp_dir.display());
    let paths = glob::glob(&pattern).expect("Failed to parse glob pattern");

    let dataframes: Vec<LazyFrame> = paths
        .map(|p| {
            let p = p.unwrap();
            let f = File::open(&p).expect("file not found");
            IpcStreamReader::new(f).finish().unwrap().lazy()
        })
        .collect();

    let df = concat(&dataframes, true, true).unwrap().collect().unwrap();
    assert_eq!(df.shape(), (expected_buffers * num_detections, 7));
}

#[test]
fn test_dataframe_agg() {
    init();

    let base_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model_path: PathBuf = base_path.join("fixtures/model.tflite");

    let expected_buffers = 512;
    let expected_columns = 21;
    let num_detections = 40;
    let max_duration = "10s";

    let pipeline_str = format!(
        "videotestsrc num-buffers={expected_buffers} \
        ! capsfilter caps=video/x-raw,width={tensor_width},height={tensor_height},format=RGB \
        ! videoscale \
        ! videoconvert \
        ! tensor_converter \
        ! capsfilter caps=other/tensors,num_tensors=1,format=static \
        ! tensor_filter framework=tensorflow2-lite model={model_file} output=4:{num_detections}:1:1,{num_detections}:1:1:1,{num_detections}:1:1:1,1:1:1:1 outputname=detection_boxes,detection_classes,detection_scores,num_detections outputtype=float32,float32,float32,float32 \
        ! tensor_decoder mode=custom-code option1=printnanny_bb_dataframe_decoder \
        ! dataframe_agg filter-threshold=0.0001 window-interval=100ms window-period=100ms max-size-duration={max_duration}",
        expected_buffers = expected_buffers,
        num_detections = num_detections,
        tensor_width = 320,
        tensor_height = 320,
        model_file = model_path.display(),
        max_duration = max_duration
    );
    println!("{}", &pipeline_str);
    let mut h = gst_check::Harness::new_parse(&pipeline_str);
    let bus = gst::Bus::new();
    let element = h.element().unwrap();
    element.set_bus(Some(&bus));
    h.play();

    let max_duration_ns = Duration::parse(max_duration).nanoseconds();

    let mut num_buffers = 0;
    while let Some(buffer) = h.pull_until_eos().unwrap() {
        let cursor = buffer.as_cursor_readable();
        let df = IpcStreamReader::new(cursor)
            .finish()
            .expect("Failed to extract dataframe");

        let (_rows, columns) = df.shape();
        println!("Pulled dataframe from buffer {:?}", df);

        assert_eq!(columns, expected_columns);

        // window should not exceed max duration
        // let max_ts = get_max_ts_offset_or_default(&df.clone().lazy(), &max_duration);
        let df = df
            .clone()
            .lazy()
            .select([(col("rt").max() - col("rt").min()).alias("rt_diff")])
            .collect()
            .unwrap();

        let ts_dif = match df.get(0).unwrap().get(0).unwrap() {
            AnyValue::Int64(v) => v.to_owned(),
            _ => 0,
        };
        assert!(max_duration_ns >= ts_dif);
        num_buffers += 1;
    }
    assert!(num_buffers == expected_buffers);
}

// requires websocket-tcp-server bin to be running, ignore in CI but keep as development helper
#[ignore]
#[test]
fn test_dataframe_agg_tcp() {
    init();

    let base_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model_path: PathBuf = base_path.join("fixtures/model.tflite");

    let expected_buffers = 512;
    let num_detections = 40;
    let max_duration = "10s";

    let pipeline_str = format!(
        "videotestsrc num-buffers={expected_buffers} \
        ! capsfilter caps=video/x-raw,width={tensor_width},height={tensor_height},format=RGB \
        ! videoscale \
        ! videoconvert \
        ! tensor_converter \
        ! capsfilter caps=other/tensors,num_tensors=1,format=static \
        ! queue leaky=2 \
        ! tensor_filter framework=tensorflow2-lite model={model_file} output=4:{num_detections}:1:1,{num_detections}:1:1:1,{num_detections}:1:1:1,1:1:1:1 outputname=detection_boxes,detection_classes,detection_scores,num_detections outputtype=float32,float32,float32,float32 \
        ! queue \
        ! tensor_decoder mode=custom-code option1=printnanny_bb_dataframe_decoder \
        ! queue \
        ! dataframe_agg filter-threshold=0.0001 window-interval=100ms window-period=100ms max-size-duration={max_duration} output-type=json-framed \
        ! tcpclientsink host=127.0.0.1 port=12345",
        expected_buffers = expected_buffers,
        num_detections = num_detections,
        tensor_width = 320,
        tensor_height = 320,
        model_file = model_path.display(),
        max_duration = max_duration
    );
    let pipeline = gst::parse_launch(&pipeline_str).unwrap();
    pipeline.set_state(gst::State::Playing).unwrap();
    let bus = pipeline.bus().unwrap();
    let mut events = vec![];
    loop {
        let msg = bus.iter_timed(gst::ClockTime::NONE).next().unwrap();

        match msg.view() {
            MessageView::Error(_) | MessageView::Eos(..) => {
                events.push(msg.clone());
                break;
            }
            // check stream related messages
            MessageView::StreamCollection(_) | MessageView::StreamsSelected(_) => {
                events.push(msg.clone())
            }
            _ => {}
        }
    }
    pipeline.set_state(gst::State::Null).unwrap();
    assert_eq!(events.len(), 1);
}
