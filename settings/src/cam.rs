use std::process::Output;

use clap::ArgMatches;
use log::{debug, error};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use gst::prelude::DeviceExt;
use gst::prelude::DeviceProviderExtManual;

use crate::error::PrintNannySettingsError;

const DEFAULT_COLORIMETRY: &str = "bt709";
const DEFAULT_PIXEL_FORMAT: &str = "YUY2";
const COMPAT_PIXEL_FORMATS: [&str; 1] = ["YUY2"];

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
            nms_threshold: 66,
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
    // #[serde(skip_serializing_if = "Option::is_none")]
    pub caps: printnanny_os_models::GstreamerCaps,
}

impl Default for CameraVideoSource {
    fn default() -> Self {
        Self {
            caps: Self::default_caps(),
            device_name: "/base/soc/i2c0mux/i2c@1/imx219@10".into(),
            label: "imx219".into(),
            index: 0,
        }
    }
}

impl CameraVideoSource {

    pub fn default_caps() -> printnanny_os_models::GstreamerCaps {
        printnanny_os_models::GstreamerCaps {
            colorimetry: DEFAULT_COLORIMETRY.into(),
            media_type: "video/x-raw".into(),
            format: DEFAULT_PIXEL_FORMAT.into(),
            width: 640,
            height: 480,
        }
    }

    pub fn camera_source_type(&self) -> printnanny_os_models::CameraSourceType {
        match &self.device_name.contains("usb") {
            true => printnanny_os_models::CameraSourceType::Usb,
            false => printnanny_os_models::CameraSourceType::Csi,
        }
    }

    pub fn list_available_caps(&self) -> Vec<printnanny_os_models::GstreamerCaps> {
        gst::init().unwrap();
        let get_factory = gst::DeviceProviderFactory::find("libcameraprovider");
        let results = if let Some(libcamera_device_provider_factory) = get_factory {
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
                            Some(caps) => {
                                caps.into_iter()
                                    .filter_map(|(s, _c)| {
                                        let height: Result<i32, gst::structure::GetError<_>> =
                                            s.get("height");
                                        let width: Result<i32, gst::structure::GetError<_>> =
                                            s.get("width");
                                        let format: Result<String, gst::structure::GetError<_>> =
                                            s.get("format");
                                        
                                        if let (Ok(height), Ok(width), Ok(format)) =
                                            (&height, &width, &format)
                                        {
                                            let media_type = s.name().into();
                                            Some(printnanny_os_models::GstreamerCaps {
                                                colorimetry: DEFAULT_COLORIMETRY.into(),
                                                height: *height,
                                                width: *width,
                                                format: format.into(),
                                                media_type,
                                            })
                                        } else {
                                            match &height {
                                                Ok(_) => (),
                                                Err(e) => {
                                                    error!(
                                                        "Failed to parse i32 from caps height={:?} with error={}",
                                                        &height, e
                                                    );
                                                }
                                            };
                                            match &width {
                                                Ok(_) => (),
                                                Err(e) => {
                                                    error!(
                                                        "Failed to parse i32 from caps width={:?} with error={}",
                                                        &width, e
                                                    );
                                                }
                                            };
                                            match &format {
                                                Ok(_) => (),
                                                Err(e) =>
                                                error!(
                                                    "Failed to read caps format={:?} with error={}",
                                                    &format, e
                                                )
                                            };
                                            None
                                        }
                                    })
                                    .collect()
                            }
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
        };
        results
            .into_iter()
            .filter(|caps| COMPAT_PIXEL_FORMATS.contains(&caps.format.as_str()))
            .collect()
    }

    pub async fn list_cameras_command_output() -> std::io::Result<Output> {
        Command::new("cam")
            .env("LIBCAMERA_LOG_LEVELS", "*:ERROR") // supress verbose output: https://libcamera.org/getting-started.html#basic-testing-with-cam-utility
            .args(["--list", "--list-properties"])
            .output()
            .await
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
                                caps: Self::default_caps(),
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

    pub async fn from_libcamera_list() -> Result<Vec<CameraVideoSource>, PrintNannySettingsError> {
        match Self::list_cameras_command_output().await {
            Ok(output) => {
                let utf8output = String::from_utf8(output.stdout)?;
                Ok(Self::parse_list_cameras_command_output(&utf8output))
            }
            Err(e) => {
                error!("Error listing libcamera devices {}", e);
                Ok(vec![])
            }
        }
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

impl From<&CameraVideoSource> for printnanny_os_models::camera::Camera {
    fn from(obj: &CameraVideoSource) -> printnanny_os_models::camera::Camera {
        let src_type = obj.camera_source_type();
        let available_caps = obj.list_available_caps();
        printnanny_os_models::camera::Camera {
            selected_caps: Box::new(obj.caps.clone()),
            available_caps,
            index: obj.index,
            label: obj.label.clone(),
            device_name: obj.device_name.clone(),
            src_type: Box::new(src_type),
        }
    }
}

impl From<printnanny_os_models::Camera> for VideoSource {
    fn from(camera: printnanny_os_models::Camera) -> VideoSource {
        match *camera.src_type {
            printnanny_os_models::CameraSourceType::Csi => {
                VideoSource::CSI(CameraVideoSource {
                    caps: *camera.selected_caps,
                    index: camera.index,
                    device_name: camera.device_name,
                    label: camera.label,
                })
            }
            printnanny_os_models::CameraSourceType::Usb => {
                VideoSource::USB(CameraVideoSource {
                    caps: *camera.selected_caps,

                    index: camera.index,
                    device_name: camera.device_name,
                    label: camera.label,
                })
            }
        }
    }
}

impl From<VideoSource> for printnanny_os_models::Camera {
    fn from(obj: VideoSource) -> printnanny_os_models::Camera {
        match &obj {
            VideoSource::CSI(camera) => printnanny_os_models::Camera {
                selected_caps: Box::new(camera.caps.clone()),
                src_type: Box::new(printnanny_os_models::CameraSourceType::Csi),
                index: camera.index,
                label: camera.label.clone(),
                device_name: camera.device_name.clone(),
                available_caps: camera.list_available_caps(),
            },
            VideoSource::USB(camera) => printnanny_os_models::Camera {
                selected_caps: Box::new(camera.caps.clone()),
                src_type: Box::new(printnanny_os_models::CameraSourceType::Usb),
                index: camera.index,
                label: camera.label.clone(),
                device_name: camera.device_name.clone(),
                available_caps: camera.list_available_caps(),
            },

            _ => todo!(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct VideoStreamSettings {
    #[serde(rename = "camera")]
    pub camera: Box<printnanny_os_models::CameraSettings>,
    #[serde(rename = "detection")]
    pub detection: Box<printnanny_os_models::DetectionSettings>,
    #[serde(rename = "hls")]
    pub hls: Box<printnanny_os_models::HlsSettings>,
    #[serde(rename = "recording")]
    pub recording: Box<printnanny_os_models::RecordingSettings>,
    #[serde(rename = "rtp")]
    pub rtp: Box<printnanny_os_models::RtpSettings>,
    #[serde(rename = "snapshot")]
    pub snapshot: Box<printnanny_os_models::SnapshotSettings>,
}

impl From<VideoStreamSettings> for printnanny_os_models::VideoStreamSettings {
    fn from(obj: VideoStreamSettings) -> printnanny_os_models::VideoStreamSettings {
        printnanny_os_models::VideoStreamSettings {
            camera: obj.camera,
            detection: obj.detection,
            hls: obj.hls,
            recording: obj.recording,
            snapshot: obj.snapshot,
            rtp: obj.rtp,
        }
    }
}

impl From<printnanny_os_models::VideoStreamSettings> for VideoStreamSettings {
    fn from(obj: printnanny_os_models::VideoStreamSettings) -> VideoStreamSettings {
        VideoStreamSettings {
            camera: obj.camera,
            detection: obj.detection,
            hls: obj.hls,
            recording: obj.recording,
            snapshot: obj.snapshot,
            rtp: obj.rtp,
        }
    }
}

impl Default for VideoStreamSettings {
    fn default() -> Self {
        let camera = Box::new(printnanny_os_models::CameraSettings {
            width: 640,
            height: 480,
            framerate_n: 16,
            framerate_d: 1,
            device_name: "/base/soc/i2c0mux/i2c@1/imx219@10".into(),
            format: DEFAULT_PIXEL_FORMAT.into(),
            label: "Raspberry Pi imx219".into(),
            colorimetry: DEFAULT_COLORIMETRY.into()
        });

        let detection = Box::new(printnanny_os_models::DetectionSettings {
            graphs: true,
            overlay: true,
            nats_server_uri: "nats://127.0.0.1:4223".into(),
            label_file: "/usr/share/printnanny/model/labels.txt".into(),
            model_file: "/usr/share/printnanny/model/model.tflite".into(),
            nms_threshold: 66,
            tensor_batch_size: 40,
            tensor_height: 320,
            tensor_width: 320,
            tensor_framerate: 2,
        });

        let hls = Box::new(printnanny_os_models::HlsSettings {
            enabled: true,
            segments: "/var/run/printnanny-hls/segment%05d.ts".into(),
            playlist: "/var/run/printnanny-hls/playlist.m3u8".into(),
            playlist_root: "/printnanny-hls/".into(),
        });

        let recording = Box::new(printnanny_os_models::RecordingSettings {
            path: "/home/printnanny/.local/share/printnanny/video".into(),
            auto_start: true,
            cloud_sync: true,
        });

        let rtp = Box::new(printnanny_os_models::RtpSettings {
            video_udp_port: 20001,
            overlay_udp_port: 20002,
        });

        let snapshot = Box::new(printnanny_os_models::SnapshotSettings {
            path: "/var/run/printnanny-snapshot/snapshot-%d.jpg".into(),
            enabled: true,
        });

        Self {
            camera,
            detection,
            hls,
            recording,
            rtp,
            snapshot,
        }
    }
}

impl VideoStreamSettings {
    pub fn gst_tensor_decoder_caps(&self) -> String {
        // Raspberry Pi Camera module v2 sensor - imx219
        // Raspberry Pi Camera module v3 sensor - imx708
        // Raspberry Pi HQ Camera Module - imx477
        if self.camera.device_name.contains("imx219") {
            format!(
                "video/x-raw,width={width},height={height},format=YUY2,interlace-mode=progressive,colorimetry=bt709",
                width = self.camera.width,
                height = self.camera.height,
            )
        }
        else {
            format!(
                "video/x-raw,width={width},height={height},format=YUY2,interlace-mode=progressive",
                width = self.camera.width,
                height = self.camera.height,
            )
        }
    }
    pub fn gst_camera_caps(&self) -> String {
        // imx219 sensor shows blue-tinted video feed when caps format/colorimetry are automatically negotiated
        // to reproduce this, run the following commands:

        // Normal colors:
        // GST_DEBUG=GST_CAPS:4 gst-launch-1.0 -vvv libcamerasrc ! 'video/x-raw,width=1280,height=720,format=YUY2' ! v4l2convert ! v4l2h264enc extra-controls="controls,repeat_sequence_header=1" ! h264parse ! 'video/x-h264,level=(string)4' ! rtph264pay ! udpsink host=localhost port=20001

        // Blue colors:
        // GST_DEBUG=GST_CAPS:4 gst-launch-1.0 -vvv libcamerasrc ! 'video/x-raw,width=1280,height=720' ! v4l2convert ! v4l2h264enc extra-controls="controls,repeat_sequence_header=1" ! h264parse ! 'video/x-h264,level=(string)4' ! rtph264pay ! udpsink host=localhost port=20001

        // So we manually specify the YUY2 format
        // NOTE this appears to be an interaction with the v4l2h264enc element, which forces upstream caps to YUY2

        // Raspberry Pi Camera module v2 sensor - imx219
        // Raspberry Pi Camera module v3 sensor - imx708
        // Raspberry Pi HQ Camera Module - imx477
        if self.camera.device_name.contains("imx219") || self.camera.device_name.contains("imx708") || self.camera.device_name.contains("imx477") {
            format!(
                "video/x-raw,width={width},height={height},framerate={framerate_n}/{framerate_d},format=YUY2,interlace-mode=progressive,colorimetry=bt709",
                width = self.camera.width,
                height = self.camera.height,
                framerate_n = self.camera.framerate_n,
                framerate_d = self.camera.framerate_d
            )
        } else {
            format!(
                "video/x-raw,width={width},height={height},framerate={framerate_n}/{framerate_d},format=YUY2,interlace-mode=progressive",
                width = self.camera.width,
                height = self.camera.height,
                framerate_n = self.camera.framerate_n,
                framerate_d = self.camera.framerate_d
            )
        }
    }

    pub async fn hotplug(mut self) -> Result<Self, PrintNannySettingsError> {
        // list available devices
        let camera_sources = CameraVideoSource::from_libcamera_list().await?;
        let selected_camera = *(self.camera.clone());
        // if no camera sources are found, return
        if camera_sources.is_empty() {
            Ok(self)
        } else {
            // is device_name among camera sources?
            for camera in camera_sources.iter() {
                // if the currently-configured device is detected, return current settings model
                if camera.device_name == selected_camera.device_name {
                    return Ok(self);
                }
            }
            // if settings model device isn't plugged in, set default to first available source
            let selected = camera_sources.first().unwrap();
            self.camera = Box::new(printnanny_os_models::CameraSettings {
                device_name: selected.device_name.clone(),
                label: selected.label.clone(),
                height: selected.caps.height,
                width: selected.caps.width,
                format: selected.caps.format.clone(),
                ..selected_camera
            });
            Ok(self)
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
                caps: CameraVideoSource::default_caps()
            }
        );
        assert_eq!(
            *result.get(1).unwrap(),
            CameraVideoSource {
                index: 2,
                label: "Logitech BRIO".into(),
                device_name: "/base/scb/pcie@7d500000/pci@0,0/usb@0,0-1:1.0-046d:085e".into(),
                caps: CameraVideoSource::default_caps()
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
                caps: CameraVideoSource::default_caps()
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
                caps: CameraVideoSource::default_caps()
            }
        )
    }

    #[test_log::test]
    fn test_parse_no_libcamera_list_command_output() {
        let result = CameraVideoSource::parse_list_cameras_command_output("");
        assert_eq!(result.len(), 0)
    }
}
