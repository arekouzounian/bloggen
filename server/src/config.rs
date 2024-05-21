use anyhow::Result;
use serde::Deserialize;

/// The configuration to be drawn from a given json file
#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub interface: String,
    pub port: String,
    pub authorized_key_file: String,
    /// Default: '.'
    pub sftp_base_dir: Option<String>,
    /// Default: stdout
    pub log_file_path: Option<String>,
}

impl ServerConfig {
    // Returns a server config object from the given json file
    pub fn try_from(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let cfg: ServerConfig = serde_json::from_str(contents.as_str())?;

        Ok(cfg)
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            interface: String::from("127.0.0.1"),
            port: String::from("2222"),
            authorized_key_file: String::from("/root/.ssh/authorized_keys"),
            sftp_base_dir: None,
            log_file_path: None,
        }
    }
}
