use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// Moonraker server config
// https://moonraker.readthedocs.io/en/latest/configuration/#server
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerServerConfig {
    // bind host address
    pub host: IpAddr,
    // HTTP port
    pub port: u16,
    // HTTPS port
    pub ssl_port: u16,
    // Unix socket used to communicate with Klippy
    pub klippy_uds_address: PathBuf,
    pub max_upload_size: u32,
}

impl Default for MoonrakerServerConfig {
    fn default() -> Self {
        Self {
            host: IpAddr::from("0.0.0.0"),
            port: 7125,
            ssl_port: 7130,
            klippy_uds_address: PathBuf::from("/var/run/klippy/klippy.sock"),
            max_upload_size: 1024,
        }
    }
}

// Moonraker file manager config
// https://moonraker.readthedocs.io/en/latest/configuration/#file_manager
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerFileManagerConfig {
    // When set to True the file manager will add uploads to the job_queue when
    // the `start_print` flag has been set.  The default if False.
    pub queue_gcode_uploads: bool,
    // When set to True gcode files will be run through a "preprocessor"
    // during metadata extraction if object tags are detected.  This preprocessor
    // replaces object tags with G-Code commands compatible with Klipper's
    // "cancel object" functionality.  Note that this process is file I/O intensive,
    // it is not recommended for usage on low resource SBCs such as a Pi Zero.
    // The default is False.
    pub enable_object_processing: bool,
    // When set to True Moonraker will generate warnings when inotify attempts
    // to add a duplicate watch or when inotify encounters an error.  On some
    // file systems inotify may not work as expected, this gives users the
    // option to suppress warnings when necessary.  The default is True.
    pub enable_inotify_warnings: bool,
}

impl Default for MoonrakerFileManagerConfig {
    fn default() -> Self {
        Self {
            queue_gcode_uploads: false,
            enable_object_processing: false,
            enable_inotify_warnings: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MoonrakerSystemServiceProvider {
    #[serde(rename = "systemd_debus")]
    SystemdDbus,
    #[serde(rename = "systemd_cli")]
    SystemdCli,
    #[serde(rename = "none")]
    None,
}

// Moonraker machine config
// https://moonraker.readthedocs.io/en/latest/configuration/#machine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerMachineConfig {
    pub provider: MoonrakerSystemServiceProvider,
    pub validate_service: bool,
    pub validate_config: bool,
    pub force_validation: bool,
}

impl Default for MoonrakerMachineConfig {
    fn default() -> Self {
        Self {
            provider: MoonrakerSystemServiceProvider::SystemdDbus,
            validate_service: false,
            validate_config: true,
            force_validation: false,
        }
    }
}

// Moonraker data (memory) store config
// https://moonraker.readthedocs.io/en/latest/configuration/#data_store
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerDataStoreConfig {
    // The maximum number of temperature values to store for each sensor.
    // applies to the "target", "power", and "fan_speed"
    pub temperature_store_size: u32,
    pub gcode_store_size: u32,
}

impl Default for MoonrakerDataStoreConfig {
    fn default() -> Self {
        Self {
            temperature_store_size: 1200, // approx 20 minutes of data @ 1 value / second
            gcode_store_size: 1000,       // maximum number of gcode lines to store in memory
        }
    }
}

// Moonraker job queue
// https://moonraker.readthedocs.io/en/latest/configuration/#job_queue
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerJobQueueConfig {
    pub load_on_startup: bool,
    pub automatic_transition: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_transition_delay: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_transition_gcode: Option<String>,
}

impl Default for MoonrakerJobQueueConfig {
    fn default() -> Self {
        Self {
            load_on_startup: false,
            automatic_transition: false,
            job_transition_delay: None,
            job_transition_gcode: None,
        }
    }
}

// Moonraker announcements
// https://moonraker.readthedocs.io/en/latest/configuration/#announcements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerAnnouncementConfig {
    pub subscriptions: Vec<String>,
    pub dev_mode: bool,
}

impl Default for MoonrakerAnnouncementConfig {
    fn default() -> Self {
        Self {
            subscriptions: vec![],
            dev_mode: false,
        }
    }
}

// Moonraker webcam
// https://moonraker.readthedocs.io/en/latest/configuration/#webcam
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerWebcamConfig {
    pub location: String,
    pub service: String,
    pub target_fps: u32,
    pub stream_url: String,
    pub snapshot_url: String,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub rotation: u32,
}

impl Default for MoonrakerWebcamConfig {
    fn default() -> Self {
        Self {
            location: "printnanny",
            service: "printnanny-vision.service",
            target_fps: 15,
            stream_url: format!("/printnanny-hls/playlist.m3u8"),
            snapshot_url: format!("/printnanny-hls/playlist.m3u8"),
            flip_horizontal: false,
            flip_vertical: false,
            rotation: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MoonrakerAuthorizationSource {
    #[serde(rename = "moonraker")]
    Moonraker,
    #[serde(rename = "ldap")]
    Ldap,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerAuthorizationConfig {
    pub login_timeout: u32,
    pub trusted_clients: Vec<String>,
    pub cors_domains: Vec<String>,
    pub force_logins: bool,
    pub default_source: MoonrakerAuthorizationSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerLDAP {
    pub ldap_host: String,
    pub ldap_port: u16,
    pub base_dn: String,
    pub bind_dn: String,
    pub bind_password: String,
    pub group_dn: String,
    pub is_active_directory: bool,
    pub user_filter: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerOctoPrintCompat {
    pub enable_ufp: bool,
    pub flip_h: bool,
    pub flip_v: bool,
    pub rotate_90: bool,
    pub stream_url: String,
    pub webcam_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonrakerConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization: Option<MoonrakerAuthorizationSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ldap: Option<MoonrakerLDAP>,
    pub octoprint_compat: Option<MoonrakerOctoPrintCompat>,
    pub announcements: MoonrakerAnnouncementConfig,
    pub data_store: MoonrakerDataStoreConfig,
    pub job_queue: MoonrakerJobQueueConfig,
    pub file_manager: MoonrakerFileManagerConfig,
    pub machine: MoonrakerMachineConfig,
    pub server: MoonrakerServerConfig,
    pub webcam: HashMap<String, MoonrakerWebcamConfig>,
}

impl Default for MoonrakerConfig {
    fn default() -> Self {
        let mut webcam = HashMap::new();
        webcam.insert("printnanny", MoonrakerWebcamConfig::default());
        Self {
            authorization: None,
            ldap: None,
            octoprint_compat: None,
            announcements: MoonrakerAnnouncementConfig::default(),
            data_store: MoonrakerDataStoreConfig::default(),
            job_queue: MoonrakerJobQueueConfig::default(),
            file_manager: MoonrakerFileManagerConfig::default(),
            machine: MoonrakerMachineConfig::default(),
            server: MoonrakerServerConfig::default(),
            webcam,
        }
    }
}
