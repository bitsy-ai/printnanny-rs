use std::process::{Command, Output};

use clap::ArgMatches;
use log::{debug, error};
use regex::Regex;
use serde::{Deserialize, Serialize};

use gst::prelude::DeviceExt;
use gst::prelude::DeviceProviderExtManual;

use printnanny_dbus::zbus;

use crate::error::PrintNannySettingsError;

#[derive(Debug, Clone, clap::ValueEnum, Deserialize, Serialize, PartialEq, Eq)]
pub enum VideoSrcType {
    File,
    CSI,
    USB,
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
pub struct CameraVideoSource {
    pub index: i32,
    pub device_name: String,
    pub label: String,
}

impl CameraVideoSource {
    pub fn default_caps() -> printnanny_asyncapi_models::GstreamerCaps {
        printnanny_asyncapi_models::GstreamerCaps {
            media_type: "video/x-raw".into(),
            format: "YUY2".into(),
            width: 640,
            height: 480,
        }
    }
    pub fn list_available_caps(&self) -> Vec<printnanny_asyncapi_models::GstreamerCaps> {
        let get_factory = gst::DeviceProviderFactory::find(" libcameraprovider");
        if let Some(libcamera_device_provider_factory) = get_factory {
            match libcamera_device_provider_factory.get() {
                Some(provider) => {
                    let devices: Vec<gst::Device> = provider
                        .devices()
                        .filter(|d| {
                            let display_name = d.display_name();
                            display_name == self.device_name
                        })
                        .collect();
                    if devices.len() > 1 {
                        error!(
                            "libcameraprovider detected multiple devices matching name: {}",
                            self.device_name
                        );
                        vec![Self::default_caps()]
                    } else if devices.len() == 1 {
                        let device = devices.first().unwrap();
                        match device.caps() {
                            Some(caps) => caps
                                .into_iter()
                                .map(|(s, _c)| {
                                    let height = s.get("height").unwrap();
                                    let width = s.get("width").unwrap();
                                    let format = s.get("format").unwrap();
                                    let media_type = s.get("media_type").unwrap();
                                    printnanny_asyncapi_models::GstreamerCaps {
                                        height,
                                        width,
                                        format,
                                        media_type,
                                    }
                                })
                                .collect(),
                            None => vec![Self::default_caps()],
                        }
                    } else {
                        error!(
                            "libcameraprovider detected 0 devices matching name {}",
                            self.device_name
                        );
                        vec![Self::default_caps()]
                    }
                }
                None => vec![Self::default_caps()],
            }
        } else {
            vec![Self::default_caps()]
        }
    }

    pub fn list_cameras_command_output() -> Result<Output, std::io::Error> {
        let output = Command::new("cam")
            .env("LIBCAMERA_LOG_LEVELS", "*:ERROR") // supress verbose output: https://libcamera.org/getting-started.html#basic-testing-with-cam-utility
            .args(["--list", "--list-properties"])
            .output()?;
        Ok(output)
    }

    pub fn parse_list_camera_line(line: &str) -> Option<CameraVideoSource> {
        let re = Regex::new(r"(\d): '(.*)' \((.*)\)").unwrap();
        match re.captures(line) {
            Some(caps) => {
                let index = caps.get(1).map(|s| s.as_str());
                let label = caps.get(2).map(|s| s.as_str());
                let device_name = caps.get(3).map(|s| s.as_str());
                debug!(
                    "parse_list_camera_line capture groups: {:#?} {:#?} {:#?}",
                    &index, &label, &device_name
                );

                match index {
                    Some(index) => match index.parse::<i32>() {
                        Ok(index) => match device_name {
                            Some(device_name) => label.map(|label| CameraVideoSource {
                                index,
                                device_name: device_name.into(),
                                label: label.into(),
                            }),
                            None => None,
                        },
                        Err(e) => {
                            error!("Failed to parse integer from {}, error: {}", &index, &e);
                            None
                        }
                    },
                    _ => None,
                }
            }
            None => None,
        }
    }

    pub fn parse_list_cameras_command_output(stdout: &str) -> Vec<CameraVideoSource> {
        let remove_str = "Available cameras:";
        let filtered = stdout.replace(remove_str, "");
        filtered
            .lines()
            .filter_map(Self::parse_list_camera_line)
            .collect()
    }

    pub fn from_libcamera_list() -> Result<Vec<CameraVideoSource>, PrintNannySettingsError> {
        let output = Self::list_cameras_command_output()?;
        let utfstdout = String::from_utf8(output.stdout)?;
        Ok(Self::parse_list_cameras_command_output(&utfstdout))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct MediaVideoSource {
    pub uri: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "src_type")]
pub enum VideoSource {
    #[serde(rename = "csi")]
    CSI(CameraVideoSource),
    #[serde(rename = "usb")]
    USB(CameraVideoSource),
    #[serde(rename = "file")]
    File(MediaVideoSource),
    #[serde(rename = "uri")]
    Uri(MediaVideoSource),
}

impl From<&CameraVideoSource> for printnanny_asyncapi_models::camera::Camera {
    fn from(obj: &CameraVideoSource) -> printnanny_asyncapi_models::camera::Camera {
        let src_type = match &obj.device_name.contains("usb") {
            true => printnanny_asyncapi_models::CameraSourceType::Usb,
            false => printnanny_asyncapi_models::CameraSourceType::Csi,
        };
        let available_caps = obj.list_available_caps();
        printnanny_asyncapi_models::camera::Camera {
            available_caps,
            index: obj.index,
            label: obj.label.clone(),
            device_name: obj.device_name.clone(),
            src_type: Box::new(src_type),
        }
    }
}

impl From<printnanny_asyncapi_models::VideoSource> for VideoSource {
    fn from(obj: printnanny_asyncapi_models::VideoSource) -> VideoSource {
        match obj {
            printnanny_asyncapi_models::VideoSource::Camera(camera) => match *camera.src_type {
                printnanny_asyncapi_models::CameraSourceType::Csi => {
                    VideoSource::CSI(CameraVideoSource {
                        index: camera.index,
                        device_name: camera.device_name,
                        label: camera.label,
                    })
                }
                printnanny_asyncapi_models::CameraSourceType::Usb => {
                    VideoSource::USB(CameraVideoSource {
                        index: camera.index,
                        device_name: camera.device_name,
                        label: camera.label,
                    })
                }
            },
            printnanny_asyncapi_models::VideoSource::PlaybackVideo(video) => {
                match *video.src_type {
                    printnanny_asyncapi_models::PlaybackSourceType::File => {
                        VideoSource::File(MediaVideoSource { uri: video.uri })
                    }
                    printnanny_asyncapi_models::PlaybackSourceType::Uri => {
                        VideoSource::Uri(MediaVideoSource { uri: video.uri })
                    }
                }
            }
        }
    }
}

//
// hls_enabled has 3 possible states:
// 1) Detect enabled/disabled based on enabled systemd services, indicated by None value
//  detect_hls_http_enabled() will be called
//
// 2) and 3) Explicitly enabled/disabled, indicated by Some(bool)
// Some(bool) -> bool
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct PrintNannyCameraSettings {
    pub preview: bool,
    pub overlay_udp_port: i32,
    pub video_udp_port: i32,
    pub video_height: i32,
    pub video_width: i32,
    pub video_framerate: i32,

    // complex types last, otherwise serde will raise TomlSerError(ValueAfterTable)
    pub detection: printnanny_asyncapi_models::PrintNannyDetectionSettings,
    pub video_src: printnanny_asyncapi_models::VideoSource,
    pub hls: printnanny_asyncapi_models::HlsSettings,
}

impl PrintNannyCameraSettings {
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

impl Default for PrintNannyCameraSettings {
    fn default() -> Self {
        let preview = false;
        let video_udp_port = 20001;
        let overlay_udp_port = 20002;

        let video_height = 480;
        let video_width = 640;
        let video_framerate = 15;
        let hls_enabled = None;
        let hls_segments = "/var/run/printnanny-hls/segment%05d.ts".into();
        let hls_playlist = "/var/run/printnanny-hls/playlist.m3u8".into();
        let hls_playlist_root = "/printnanny-hls/".into();

        let hls = printnanny_asyncapi_models::HlsSettings {
            hls_enabled,
            hls_segments,
            hls_playlist,
            hls_playlist_root,
        };

        let video_src =
            printnanny_asyncapi_models::VideoSource::Camera(printnanny_asyncapi_models::Camera {
                available_caps: vec![CameraVideoSource::default_caps()],
                device_name: "/base/soc/i2c0mux/i2c@1/imx219@10".into(),
                label: "imx219".into(),
                index: 0,
                src_type: Box::new(printnanny_asyncapi_models::CameraSourceType::Csi),
            });

        let detection = printnanny_asyncapi_models::PrintNannyDetectionSettings {
            nats_server_uri: "nats://127.0.0.1:4223".into(),
            label_file: "/usr/share/printnanny/model/labels.txt".into(),
            model_file: "/usr/share/printnanny/model/model.tflite".into(),
            nms_threshold: 50,
            tensor_batch_size: 40,
            tensor_height: 320,
            tensor_width: 320,
            tensor_framerate: 2,
        };

        Self {
            video_src,
            video_height,
            video_width,
            video_framerate,
            video_udp_port,
            overlay_udp_port,
            preview,
            hls,
            detection,
        }
    }
}

impl From<printnanny_asyncapi_models::PrintNannyCameraSettings> for PrintNannyCameraSettings {
    fn from(obj: printnanny_asyncapi_models::PrintNannyCameraSettings) -> PrintNannyCameraSettings {
        PrintNannyCameraSettings {
            overlay_udp_port: obj.overlay_udp_port,
            video_udp_port: obj.video_udp_port,
            preview: obj.preview,
            video_framerate: obj.video_framerate,
            video_height: obj.video_height,
            video_width: obj.video_width,
            detection: *obj.detection,
            hls: *obj.hls,
            video_src: *obj.video_src,
        }
    }
}

impl From<PrintNannyCameraSettings> for printnanny_asyncapi_models::PrintNannyCameraSettings {
    fn from(obj: PrintNannyCameraSettings) -> printnanny_asyncapi_models::PrintNannyCameraSettings {
        printnanny_asyncapi_models::PrintNannyCameraSettings {
            overlay_udp_port: obj.overlay_udp_port,
            video_udp_port: obj.video_udp_port,
            preview: obj.preview,
            video_framerate: obj.video_framerate,
            video_height: obj.video_height,
            video_width: obj.video_width,
            detection: Box::new(obj.detection),
            hls: Box::new(obj.hls),
            video_src: Box::new(obj.video_src),
        }
    }
}

impl From<&ArgMatches> for PrintNannyCameraSettings {
    fn from(args: &ArgMatches) -> Self {
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

        let tensor_framerate: i32 = args
            .value_of_t::<i32>("tensor_framerate")
            .expect("--tensor-framerate must be an integer");

        let nms_threshold: i32 = args
            .value_of_t::<i32>("nms_threshold")
            .expect("--nms-threshold must be an integer");

        let preview = args.is_present("preview");

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

        let detection = printnanny_asyncapi_models::PrintNannyDetectionSettings {
            label_file,
            model_file,
            nats_server_uri,
            nms_threshold,
            tensor_batch_size,
            tensor_height,
            tensor_width,
            tensor_framerate,
        };

        let hls_enabled = match args.is_present("hls_http_enabled") {
            true => Some(true),
            false => None,
        };

        let hls = printnanny_asyncapi_models::HlsSettings {
            hls_enabled,
            hls_segments,
            hls_playlist,
            hls_playlist_root,
        };

        Self {
            detection,
            preview,
            video_height,
            video_width,
            video_framerate,
            video_udp_port,
            overlay_udp_port,
            hls,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MULTIPLE_CAMERAS: &str = r#"Available cameras:
1: 'imx219' (/base/soc/i2c0mux/i2c@1/imx219@10)
2: 'Logitech BRIO' (/base/scb/pcie@7d500000/pci@0,0/usb@0,0-1:1.0-046d:085e)"#;

    const ONE_CSI_CAMERA: &str = r#"Available cameras:
1: 'imx219' (/base/soc/i2c0mux/i2c@1/imx219@10)"#;

    const ONE_USB_CAMERA: &str = r#"Available cameras:
1: 'Logitech BRIO' (/base/scb/pcie@7d500000/pci@0,0/usb@0,0-1:1.0-046d:085e)"#;

    #[test_log::test]
    fn test_parse_multiple_libcamera_list_command_output() {
        let result = CameraVideoSource::parse_list_cameras_command_output(MULTIPLE_CAMERAS);

        assert_eq!(
            *result.get(0).unwrap(),
            CameraVideoSource {
                index: 1,
                label: "imx219".into(),
                device_name: "/base/soc/i2c0mux/i2c@1/imx219@10".into(),
            }
        );
        assert_eq!(
            *result.get(1).unwrap(),
            CameraVideoSource {
                index: 2,
                label: "Logitech BRIO".into(),
                device_name: "/base/scb/pcie@7d500000/pci@0,0/usb@0,0-1:1.0-046d:085e".into(),
            }
        )
    }
    #[test_log::test]
    fn test_parse_one_csi_libcamera_list_command_output() {
        let result = CameraVideoSource::parse_list_cameras_command_output(ONE_CSI_CAMERA);

        assert_eq!(
            *result.get(0).unwrap(),
            CameraVideoSource {
                index: 1,
                label: "imx219".into(),
                device_name: "/base/soc/i2c0mux/i2c@1/imx219@10".into(),
            }
        );
    }
    #[test_log::test]
    fn test_parse_one_usb_libcamera_list_command_output() {
        let result = CameraVideoSource::parse_list_cameras_command_output(ONE_USB_CAMERA);
        assert_eq!(
            *result.get(0).unwrap(),
            CameraVideoSource {
                index: 1,
                label: "Logitech BRIO".into(),
                device_name: "/base/scb/pcie@7d500000/pci@0,0/usb@0,0-1:1.0-046d:085e".into(),
            }
        )
    }

    #[test_log::test]
    fn test_parse_no_libcamera_list_command_output() {
        let result = CameraVideoSource::parse_list_cameras_command_output("");
        assert_eq!(result.len(), 0)
    }
}
