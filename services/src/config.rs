use figment::error::Result;
use figment::providers::{Env, Format, Json, Serialized, Toml};
use figment::value::{Dict, Map, Value};
use figment::{Figment, Metadata, Profile, Provider};
use glob::glob;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::fs;

use printnanny_api_client::apis::configuration::Configuration as ReqwestConfig;
use printnanny_api_client::models;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiConfig {
    pub base_path: String,
    pub bearer_access_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JanusConfig {
    pub admin_secret: String,
    pub token: String,
    pub admin_base_path: String,
    pub admin_http_port: i32,
    pub admin_https_port: i32,
    pub base_path: String,
    pub http_port: i32,
    pub https_port: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MQTTConfig {
    pub ca_certs: String,
    pub private_key: String,
    pub public_key: String,
    pub fingerprint: String,
    pub fingerprint_algorithm: String,
    pub cipher: String,
    pub length: i32,
}

impl Default for MQTTConfig {
    fn default() -> Self {
        Self {
            private_key: "/opt/printnanny/default/keys/ec_private.pem".into(),
            public_key: "/opt/printnanny/default/keys/ec_public.pem".into(),
            fingerprint: "".into(),
            fingerprint_algorithm: "md5".into(),
            ca_certs: "/opt/printnanny/default/ca-certificates".into(),
            cipher: "secp256r1".into(),
            length: 4096,
        }
    }
}

impl Default for JanusConfig {
    fn default() -> Self {
        Self {
            admin_secret: "debug".into(),
            token: "debug".into(),
            admin_base_path: "/admin".into(),
            admin_http_port: 7088,
            admin_https_port: 7089,
            base_path: "/janus".into(),
            http_port: 8088,
            https_port: 8089,
        }
    }
}

impl JanusConfig {
    pub fn admin_http_url(&self) -> String {
        format!(
            "http://localhost:{}{}",
            self.admin_http_port, self.admin_base_path
        )
    }
    pub fn admin_https_url(&self) -> String {
        let hostname = sys_info::hostname().unwrap_or("localhost".into());
        format!(
            "https://{}:{}{}",
            hostname, self.admin_https_port, self.admin_base_path
        )
    }
    pub fn http_url(&self) -> String {
        format!("http://localhost:{}{}", self.http_port, self.base_path)
    }
    pub fn https_url(&self) -> String {
        let hostname = sys_info::hostname().unwrap_or("localhost".into());
        format!("https://{}:{}{}", hostname, self.https_port, self.base_path)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PrintNannyConfig {
    pub path: String,
    pub api: ApiConfig,
    pub janus: JanusConfig,
    pub mqtt: MQTTConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<models::Device>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<models::User>,
}
pub struct ConfigError {}

impl From<&ApiConfig> for ReqwestConfig {
    fn from(api: &ApiConfig) -> ReqwestConfig {
        ReqwestConfig {
            base_path: api.base_path.to_string(),
            bearer_access_token: api.bearer_access_token.clone(),
            ..ReqwestConfig::default()
        }
    }
}

impl Default for PrintNannyConfig {
    fn default() -> Self {
        let api = ApiConfig {
            base_path: "https://print-nanny.com".into(),
            bearer_access_token: None,
        };
        let path = "/opt/printnanny/default".into();
        let janus = JanusConfig::default();
        let mqtt = MQTTConfig::default();
        PrintNannyConfig {
            api,
            janus,
            mqtt,
            path,
            device: None,
            user: None,
        }
    }
}

impl PrintNannyConfig {
    // See example: https://docs.rs/figment/latest/figment/index.html#extracting-and-profiles
    // Note the `nested` option on both `file` providers. This makes each
    // top-level dictionary act as a profile
    pub fn new(config: Option<&str>) -> Result<Self> {
        let figment = Self::figment(config);
        let result = figment.extract()?;
        info!("Initialized config {:?}", result);
        Ok(result)
    }

    // intended for use with Rocket's figmment
    pub fn from_figment(config: Option<&str>, figment: Figment) -> Figment {
        return figment.merge(Self::figment(config));
    }

    pub fn figment(config: Option<&str>) -> Figment {
        let result = Figment::from(Self {
            // profile,
            ..Self::default()
        })
        .merge(Toml::file(Env::var_or(
            "PRINTNANNY_CONFIG",
            "PrintNanny.toml",
        )))
        .merge(Env::prefixed("PRINTNANNY_").global());

        let result = match config {
            Some(c) => result.clone().merge(Toml::file(c)),
            None => result,
        };

        info!("Loaded config from profile {:?}", result.profile());
        let path: String = result
            .find_value("path")
            .unwrap_or(Value::from(Self::default().path))
            .deserialize::<String>()
            .unwrap();

        let toml_glob = format!("{}/*.toml", &path);
        let json_glob = format!("{}/*.json", &path);

        let result = Self::read_path_glob::<Json>(&json_glob, result);
        let result = Self::read_path_glob::<Toml>(&toml_glob, result);

        result
    }

    fn read_path_glob<T: 'static + figment::providers::Format>(
        pattern: &str,
        figment: Figment,
    ) -> Figment {
        info!("Merging config from {:?}", &pattern);
        let mut result = figment.clone();
        for entry in glob(&pattern).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    info!("Merging config from {:?}", &path);
                    result = result.clone().merge(T::file(path))
                }
                Err(e) => error!("{:?}", e),
            }
        }
        result
    }

    pub fn save(self) -> String {
        let msg = format!("Failed to serialize {:?}", &self);
        let content = serde_json::to_string(&self).expect(&msg);
        let filename = format!("{}/{}", &self.path, "License.json");
        let msg = format!("Unable to write file: {}", &filename);
        fs::write(&filename, content).expect(&msg);
        info!(
            "Wrote user={:?} device={:?} config to {}",
            &self.device, &self.user, &filename
        );
        filename.to_string()
    }

    /// Extract a `Config` from `provider`, panicking if extraction fails.
    ///
    /// # Panics
    ///
    /// If extraction fails, prints an error message indicating the failure and
    /// panics. For a version that doesn't panic, use [`Config::try_from()`].
    ///
    /// # Example
    pub fn from<T: Provider>(provider: T) -> Self {
        Self::try_from(provider).unwrap_or_else(|e| {
            error!("{:?}", e);
            panic!("aborting due to configuration error(s)")
        })
    }

    /// Attempts to extract a `Config` from `provider`, returning the result.
    ///
    /// # Example
    pub fn try_from<T: Provider>(provider: T) -> Result<Self> {
        let figment = Figment::from(provider);
        let config = figment.extract::<Self>()?;
        Ok(config)
    }
}

impl Provider for PrintNannyConfig {
    fn metadata(&self) -> Metadata {
        Metadata::named("Print Nanny Config")
    }

    fn data(&self) -> Result<Map<Profile, Dict>> {
        let map: Map<Profile, Dict> = Serialized::defaults(self).data()?;
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test_log::test]
    fn test_env_merged() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "PrintNanny.toml",
                r#"
            name = "default"
            path = "/opt/printnanny/default"
            
            [api]
            base_path = "https://print-nanny.com"
            "#,
            )?;
            let figment = PrintNannyConfig::figment(None);
            let config: PrintNannyConfig = figment.extract()?;
            assert_eq!(
                config.api,
                ApiConfig {
                    base_path: "https://print-nanny.com".into(),
                    bearer_access_token: None
                }
            );

            jail.set_env("PRINTNANNY_API.BEARER_ACCESS_TOKEN", "secret");
            let figment = PrintNannyConfig::figment(None);
            let config: PrintNannyConfig = figment.extract()?;
            assert_eq!(
                config.api,
                ApiConfig {
                    base_path: "https://print-nanny.com".into(),
                    bearer_access_token: Some("secret".into())
                }
            );
            Ok(())
        });
    }

    #[test_log::test]
    fn test_custom_config_file() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "Local.toml",
                r#"
            name = "local"
            path = "/home/leigh/projects/print-nanny-cli/.tmp"
            
            [api]
            base_path = "http://aurora:8000"
            "#,
            )?;
            jail.set_env("PRINTNANNY_CONFIG", "Local.toml");

            let figment = PrintNannyConfig::figment(None);
            let config: PrintNannyConfig = figment.extract()?;

            let base_path = "http://aurora:8000".into();
            let path: String = "/home/leigh/projects/print-nanny-cli/.tmp".into();
            assert_eq!(config.path, path);
            assert_eq!(config.api.base_path, base_path);

            assert_eq!(
                config.api,
                ApiConfig {
                    base_path: base_path,
                    bearer_access_token: None
                }
            );
            Ok(())
        });
    }

    #[test_log::test]
    fn test_custom_json_toml_glob() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "Base.toml",
                r#"
            path = ".tmp"
            
            [api]
            base_path = "https://print-nanny.com"
            "#,
            )?;
            let tmp_directory = jail.directory();
            std::fs::create_dir(tmp_directory.join(".tmp")).unwrap();
            jail.create_file(
                ".tmp/ApiConfig.toml",
                r#"
            [api]
            base_path = "http://aurora:8000"
            bearer_access_token = "abc123"
            "#,
            )?;
            // jail.create_file(
            //     ".tmp/device.json",
            //     r#"
            // "#,
            // )?;
            jail.set_env("PRINTNANNY_CONFIG", "Base.toml");
            let figment = PrintNannyConfig::figment(None);
            let config: PrintNannyConfig = figment.extract()?;
            info!("Read config {:?}", config);
            // assert_eq!(config.api_config.bearer_access_token, "local");
            // assert_eq!(config.device.unwrap().id, 1);
            assert_eq!(
                config.api,
                ApiConfig {
                    base_path: "http://aurora:8000".into(),
                    bearer_access_token: Some("abc123".into())
                }
            );
            Ok(())
        });
    }
}
