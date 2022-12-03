use std::collections::HashMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use figment::providers::Env;
use log::{debug, info};
use serde::{Deserialize, Serialize};

use printnanny_dbus::zbus;
use printnanny_dbus::zbus_systemd;

use super::vcs::{VersionControlledSettings, VersionControlledSettingsError};
use crate::settings::SettingsFormat;

pub const MOONRAKER_INSTALL_DIR: &str = "/home/printnanny/.moonraker";
pub const MOONRAKER_VENV: &str = "/home/printnanny/moonraker-venv";
pub const DEFAULT_MOONRAKER_SETTINGS_FILE: &str =
    "/home/printnanny/.config/printnanny/setttings/moonraker/moonraker.conf";

// Moonraker server config
// https://moonraker.readthedocs.io/en/latest/configuration/#server
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonrakerServerSettings {
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

impl Default for MoonrakerServerSettings {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".parse().unwrap(),
            port: 7125,
            ssl_port: 7130,
            klippy_uds_address: PathBuf::from("/var/run/klippy/klippy.sock"),
            max_upload_size: 1024,
        }
    }
}

// Moonraker file manager config
// https://moonraker.readthedocs.io/en/latest/configuration/#file_manager
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonrakerFileManagerSettings {
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

impl Default for MoonrakerFileManagerSettings {
    fn default() -> Self {
        Self {
            queue_gcode_uploads: false,
            enable_object_processing: false,
            enable_inotify_warnings: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonrakerMachineSettings {
    pub provider: MoonrakerSystemServiceProvider,
    pub validate_service: bool,
    pub validate_config: bool,
    pub force_validation: bool,
}

impl Default for MoonrakerMachineSettings {
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonrakerDataStoreSettings {
    // The maximum number of temperature values to store for each sensor.
    // applies to the "target", "power", and "fan_speed"
    pub temperature_store_size: u32,
    pub gcode_store_size: u32,
}

impl Default for MoonrakerDataStoreSettings {
    fn default() -> Self {
        Self {
            temperature_store_size: 1200, // approx 20 minutes of data @ 1 value / second
            gcode_store_size: 1000,       // maximum number of gcode lines to store in memory
        }
    }
}

// Moonraker job queue
// https://moonraker.readthedocs.io/en/latest/configuration/#job_queue
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonrakerJobQueueSettings {
    pub load_on_startup: bool,
    pub automatic_transition: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_transition_delay: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_transition_gcode: Option<String>,
}

impl Default for MoonrakerJobQueueSettings {
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonrakerAnnouncementSettings {
    pub subscriptions: Vec<String>,
    pub dev_mode: bool,
}

impl Default for MoonrakerAnnouncementSettings {
    fn default() -> Self {
        Self {
            subscriptions: vec![],
            dev_mode: false,
        }
    }
}

// Moonraker webcam
// https://moonraker.readthedocs.io/en/latest/configuration/#webcam
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonrakerWebcamSettings {
    pub location: String,
    pub service: String,
    pub target_fps: u32,
    pub stream_url: String,
    pub snapshot_url: String,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub rotation: u32,
}

impl Default for MoonrakerWebcamSettings {
    fn default() -> Self {
        Self {
            location: "printnanny".into(),
            service: "printnanny-vision.service".into(),
            target_fps: 15,
            stream_url: "/printnanny-hls/playlist.m3u8".into(),
            snapshot_url: "/printnanny-hls/playlist.m3u8".into(),
            flip_horizontal: false,
            flip_vertical: false,
            rotation: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoonrakerAuthorizationSource {
    #[serde(rename = "moonraker")]
    Moonraker,
    #[serde(rename = "ldap")]
    Ldap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonrakerAuthorizationSettings {
    pub login_timeout: u32,
    pub trusted_clients: Vec<String>,
    pub cors_domains: Vec<String>,
    pub force_logins: bool,
    pub default_source: MoonrakerAuthorizationSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonrakerOctoPrintCompat {
    pub enable_ufp: bool,
    pub flip_h: bool,
    pub flip_v: bool,
    pub rotate_90: bool,
    pub stream_url: String,
    pub webcam_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonrakerMqttSettings {
    pub address: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub mqtt_protocol: String,
    pub enable_moonraker_api: bool,
    pub instance_name: String,
    pub status_objects: Vec<String>,
    pub default_qos: u8,
    pub api_qos: u8,
}

impl Default for MoonrakerMqttSettings {
    fn default() -> Self {
        let hostname = sys_info::hostname().unwrap_or_else(|_| "localhost".to_string());

        Self {
            address: "mqtt.live.printnanny.ai".into(),
            port: 1883,
            username: "{secrets.mqtt_credentials.username}".into(), // jinja template string, see Moonraker [secrets] documentation: https://moonraker.readthedocs.io/en/latest/configuration/#jinja2-templates
            password: "{secrets.mqtt_credentials.password}".into(), // jinja template string, see Moonraker [secrets] documentation: https://moonraker.readthedocs.io/en/latest/configuration/#jinja2-templates
            mqtt_protocol: "v3.1.1".into(),
            enable_moonraker_api: true,
            instance_name: hostname,
            status_objects: vec![],
            default_qos: 0,
            api_qos: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonrakerMqttCredentials {
    username: String,
    password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonrakerSecretsSettings {
    pub mqtt_credentials: MoonrakerMqttCredentials,
}

// based on: https://moonraker.readthedocs.io/en/latest/configuration/
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonrakerCfg {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization: Option<MoonrakerAuthorizationSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ldap: Option<MoonrakerLDAP>,
    pub secrets: Option<MoonrakerSecretsSettings>,
    pub octoprint_compat: Option<MoonrakerOctoPrintCompat>,
    pub announcements: MoonrakerAnnouncementSettings,
    pub data_store: MoonrakerDataStoreSettings,
    pub job_queue: MoonrakerJobQueueSettings,
    pub file_manager: MoonrakerFileManagerSettings,
    pub machine: MoonrakerMachineSettings,
    pub server: MoonrakerServerSettings,
    pub mqtt: MoonrakerMqttSettings,
    pub webcam: HashMap<String, MoonrakerWebcamSettings>,
}

impl Default for MoonrakerCfg {
    fn default() -> Self {
        let mut webcam = HashMap::new();
        webcam.insert("printnanny".into(), MoonrakerWebcamSettings::default());
        Self {
            authorization: None,
            ldap: None,
            octoprint_compat: None,
            secrets: None,
            announcements: MoonrakerAnnouncementSettings::default(),
            data_store: MoonrakerDataStoreSettings::default(),
            job_queue: MoonrakerJobQueueSettings::default(),
            file_manager: MoonrakerFileManagerSettings::default(),
            machine: MoonrakerMachineSettings::default(),
            server: MoonrakerServerSettings::default(),
            mqtt: MoonrakerMqttSettings::default(),
            webcam,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoonrakerSettings {
    pub enabled: bool,
    pub install_dir: PathBuf,
    pub settings_file: PathBuf,
    pub settings_format: SettingsFormat,
    pub venv: PathBuf,
}

impl Default for MoonrakerSettings {
    fn default() -> Self {
        let install_dir: PathBuf = MOONRAKER_INSTALL_DIR.into();
        let settings_file = PathBuf::from(Env::var_or(
            "MOONRAKER_SETTINGS_FILE",
            DEFAULT_MOONRAKER_SETTINGS_FILE,
        ));
        Self {
            settings_file,
            install_dir,
            enabled: false,
            venv: MOONRAKER_VENV.into(),
            settings_format: SettingsFormat::Ini,
        }
    }
}

#[async_trait]
impl VersionControlledSettings for MoonrakerSettings {
    type SettingsModel = MoonrakerSettings;
    fn from_dir(settings_dir: &Path) -> Self {
        let settings_file = settings_dir.join("moonraker/moonraker.conf");
        Self {
            settings_file,
            ..Self::default()
        }
    }
    fn get_settings_format(&self) -> SettingsFormat {
        self.settings_format
    }
    fn get_settings_file(&self) -> PathBuf {
        self.settings_file.clone()
    }
    async fn pre_save(&self) -> Result<(), VersionControlledSettingsError> {
        debug!("Running KlipperSettings pre_save hook");
        // stop OctoPrint serviice
        let connection = zbus::Connection::system().await?;

        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let job = proxy
            .stop_unit("klipper.service".to_string(), "replace".to_string())
            .await?;
        info!("Stopped klipper.service, job: {:?}", job);
        Ok(())
    }

    async fn post_save(&self) -> Result<(), VersionControlledSettingsError> {
        debug!("Running KlipperSettings post_save hook");
        // start OctoPrint service
        let connection = zbus::Connection::system().await?;
        let proxy = zbus_systemd::systemd1::ManagerProxy::new(&connection).await?;
        let job = proxy
            .start_unit("klipper.service".into(), "replace".into())
            .await?;
        info!("Started klipper.service, job: {:?}", job);

        Ok(())
    }
    fn validate(&self) -> Result<(), VersionControlledSettingsError> {
        todo!("OctoPrintSettings validate hook is not yet implemented");
    }
}
