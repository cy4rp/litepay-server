use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_db_url")]
    pub database_url: String,
    #[serde(default)]
    pub ln_backend: LnBackendConfig,
    #[serde(default = "default_site_title")]
    pub site_title: String,
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LnBackendConfig {
    #[default]
    Mock,
    Lnd {
        url: String,
        macaroon: String,
        #[serde(default)]
        tls_cert: Option<String>,
    },
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    9000
}
fn default_db_url() -> String {
    "sqlite://litepay.db?mode=rwc".to_string()
}
fn default_site_title() -> String {
    "LitePay Server".to_string()
}

impl Config {
    pub fn load(path: Option<&str>) -> anyhow::Result<Self> {
        let path = path.unwrap_or("litepay.toml");
        if Path::new(path).exists() {
            let contents = std::fs::read_to_string(path)?;
            let config: Config = toml::from_str(&contents)?;
            Ok(config)
        } else {
            Ok(Self {
                host: default_host(),
                port: default_port(),
                database_url: default_db_url(),
                ln_backend: LnBackendConfig::default(),
                site_title: default_site_title(),
            })
        }
    }
}
