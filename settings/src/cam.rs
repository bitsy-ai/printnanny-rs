use clap::ArgMatches;

use serde::{Deserialize, Serialize};

use printnanny_dbus::zbus;

#[derive(Debug, Clone, clap::ValueEnum, Deserialize, Serialize, PartialEq, Eq)]
pub enum VideoSrcType {
    File,
    Device,
    Uri,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct TfliteModelSettings {
    pub label_file: String,
    pub model_file: String,
    pub nms_threshold: i32,
    pub tensor_batch_size: i32,
    pub tensor_channels: i32,
    pub tensor_height: i32,
    pub tensor_width: i32,
    pub tensor_framerate: i32,
}

impl Default for TfliteModelSettings {
    fn default() -> Self {
        Self {
            label_file: "/usr/share/printnanny/model/labels.txt".into(),
            model_file: "/usr/share/printnanny/model/model.tflite".into(),
            nms_threshold: 50,
            tensor_batch_size: 40,
            tensor_channels: 3,
            tensor_height: 320,
            tensor_width: 320,
            tensor_framerate: 2,
        }
    }
}

impl From<&ArgMatches> for TfliteModelSettings {
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
            .value_of_t::<i32>("tensor_batch_size")
            .expect("--tensor-batch-size must be an integer");

        let tensor_height: i32 = args
            .value_of_t::<i32>("tensor_height")
            .expect("--tensor-height must be an integer");

        let tensor_width: i32 = args
            .value_of_t::<i32>("tensor_width")
            .expect("--tensor-width must be an integer");

        let tensor_channels: i32 = args
            .value_of_t::<i32>("tensor_channels")
            .expect("--tensor-channels must be an integer");

        let tensor_framerate: i32 = args
            .value_of_t::<i32>("tensor_framerate")
            .expect("--tensor-framerate must be an integer");

        let nms_threshold: i32 = args
            .value_of_t::<i32>("nms_threshold")
            .expect("--nms-threshold must be an integer");

        Self {
            label_file,
            model_file,
            nms_threshold,
            tensor_batch_size,
            tensor_channels,
            tensor_height,
            tensor_width,
            tensor_framerate,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct PrintNannyCamSettings {
    pub video_src: String,
    pub preview: bool,
    pub nats_server_uri: String,
    pub overlay_udp_port: i32,
    pub video_udp_port: i32,
    pub video_height: i32,
    pub video_width: i32,
    pub video_framerate: i32,
    pub hls_segments: String,
    pub hls_playlist: String,
    pub hls_playlist_root: String,

    //
    // hls_http has 3 possible states:
    // 1) Detect enabled/disabled based on enabled systemd services, indicated by None value
    //  detect_hls_http_enabled() will be called
    //
    // 2) and 3) Explicitly enabled/disabled, indicated by Some(bool)
    // Some(bool) -> bool
    pub hls_http_enabled: Option<bool>,
    // complex types last, otherwise serde will raise TomlSerError(ValueAfterTable)
    pub video_src_type: VideoSrcType,
    pub tflite_model: TfliteModelSettings,
}

impl PrintNannyCamSettings {
    pub async fn detect_hls_http_enabled(&self) -> Result<bool, zbus::Error> {
        let connection = zbus::Connection::system().await?;
        let proxy = printnanny_dbus::zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let unit_path = proxy
            .get_unit_file_state("octoprint.service".into())
            .await?;

        let result = &unit_path == "enabled";
        Ok(result)
    }
}

impl Default for PrintNannyCamSettings {
    fn default() -> Self {
        let video_src = "/dev/video0".into();
        let preview = false;
        let tflite_model = TfliteModelSettings::default();
        let video_udp_port = 20001;
        let overlay_udp_port = 20002;

        let video_src_type = VideoSrcType::Device;
        let video_height = 480;
        let video_width = 640;
        let video_framerate = 15;
        let hls_http_enabled = None;
        let hls_segments = "/var/run/printnanny-hls/segment%05d.ts".into();
        let hls_playlist = "/var/run/printnanny-hls/playlist.m3u8".into();
        let hls_playlist_root = "/printnanny-hls/".into();

        let nats_server_uri = "nats://127.0.0.1:4223".into();

        Self {
            video_src,
            tflite_model,
            video_src_type,
            video_height,
            video_width,
            video_framerate,
            video_udp_port,
            overlay_udp_port,
            preview,
            hls_http_enabled,
            hls_segments,
            hls_playlist,
            hls_playlist_root,
            nats_server_uri,
        }
    }
}

impl From<&ArgMatches> for PrintNannyCamSettings {
    fn from(args: &ArgMatches) -> Self {
        let tflite_model = TfliteModelSettings::from(args);

        let video_src_type: &VideoSrcType = args
            .get_one::<VideoSrcType>("video_src_type")
            .expect("--video-src-type");

        let video_src = args
            .value_of("video_src")
            .expect("--video-src is required")
            .into();
        let video_height: i32 = args
            .value_of_t::<i32>("video_height")
            .expect("--video-height must be an integer");

        let video_framerate: i32 = args
            .value_of_t::<i32>("video_framerate")
            .expect("--video-framerate must be an integer");

        let video_width: i32 = args
            .value_of_t::<i32>("video_width")
            .expect("--video-width must be an integer");

        let video_udp_port: i32 = args
            .value_of_t("video_udp_port")
            .expect("--video-udp-port must be an integer");

        let overlay_udp_port: i32 = args
            .value_of_t("overlay_udp_port")
            .expect("--overlay-udp-port must be an integer");

        let preview = args.is_present("preview");

        let hls_http_enabled = match args.is_present("hls_http_enabled") {
            true => Some(true),
            false => None,
        };

        let hls_segments: String = args
            .value_of("hls_segments")
            .expect("--hls-segments is required")
            .into();

        let hls_playlist: String = args
            .value_of("hls_playlist")
            .expect("--hls-playlist is required")
            .into();

        let hls_playlist_root: String = args
            .value_of("hls_playlist_root")
            .expect("--hls-playlist-root is required")
            .into();

        let nats_server_uri: String = args
            .value_of("nats_server_uri")
            .expect("--nats-server-uri is required")
            .into();

        Self {
            tflite_model,
            preview,
            video_src,
            video_height,
            video_width,
            video_framerate,
            video_src_type: video_src_type.clone(),
            video_udp_port,
            overlay_udp_port,
            hls_http_enabled,
            hls_segments,
            hls_playlist,
            hls_playlist_root,
            nats_server_uri,
        }
    }
}
