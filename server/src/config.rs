use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub cors: CorsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub json: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            json: false,
        }
    }
}

/// CORS configuration for the HTTP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Whether to allow all origins (useful for development).
    /// Set to false in production and list specific origins in `allowed_origins`.
    #[serde(default)]
    pub allow_any_origin: bool,
    /// Explicit list of allowed origins (used when `allow_any_origin` is false).
    /// Example: ["https://myblog.example.com"]
    #[serde(default)]
    pub allowed_origins: Vec<String>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allow_any_origin: true,
            allowed_origins: vec![],
        }
    }
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_max_connections() -> u32 {
    10
}

fn default_min_connections() -> u32 {
    2
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&contents)?;
        Ok(config)
    }

    pub fn database_url(&self) -> &str {
        &self.database.url
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_defaults() {
        let config_json = r#"{
            "server": {},
            "database": {
                "url": "postgres://localhost/test"
            }
        }"#;

        let config: Config = serde_json::from_str(config_json).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.database.max_connections, 10);
        assert_eq!(config.database.min_connections, 2);
        assert_eq!(config.logging.level, "info");
        assert!(!config.logging.json);
    }

    #[test]
    fn test_config_custom_values() {
        let config_json = r#"{
            "server": {
                "host": "0.0.0.0",
                "port": 8080
            },
            "database": {
                "url": "postgres://example.com/mydb",
                "max_connections": 20,
                "min_connections": 5
            },
            "logging": {
                "level": "debug",
                "json": true
            }
        }"#;

        let config: Config = serde_json::from_str(config_json).unwrap();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.url, "postgres://example.com/mydb");
        assert_eq!(config.database.max_connections, 20);
        assert_eq!(config.database.min_connections, 5);
        assert_eq!(config.logging.level, "debug");
        assert!(config.logging.json);
    }

    #[test]
    fn test_config_from_file() {
        let config_json = r#"{
            "server": {
                "host": "127.0.0.1",
                "port": 3000
            },
            "database": {
                "url": "postgres://localhost/bloggen"
            }
        }"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_json.as_bytes()).unwrap();

        let config = Config::from_file(temp_file.path()).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.database_url(), "postgres://localhost/bloggen");
    }

    #[test]
    fn test_config_missing_file() {
        let result = Config::from_file("/nonexistent/path/config.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_invalid_json() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"not valid json").unwrap();

        let result = Config::from_file(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_cors_defaults_to_allow_any_origin() {
        let config_json = r#"{
            "server": {},
            "database": { "url": "postgres://localhost/test" }
        }"#;
        let config: Config = serde_json::from_str(config_json).unwrap();
        assert!(config.cors.allow_any_origin, "Default CORS should allow any origin");
        assert!(config.cors.allowed_origins.is_empty());
    }

    #[test]
    fn test_cors_restricted_origins() {
        let config_json = r#"{
            "server": {},
            "database": { "url": "postgres://localhost/test" },
            "cors": {
                "allow_any_origin": false,
                "allowed_origins": ["https://myblog.example.com"]
            }
        }"#;
        let config: Config = serde_json::from_str(config_json).unwrap();
        assert!(!config.cors.allow_any_origin);
        assert_eq!(config.cors.allowed_origins, ["https://myblog.example.com"]);
    }
}
