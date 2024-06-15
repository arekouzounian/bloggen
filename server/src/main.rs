//
// Copyright © 2024 Arek Ouzounian <arek@arekouzounian.com>
//
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use config::ServerConfig;
use russh::server::{Msg, Server as _, Session};
use russh::{Channel, ChannelId};
use russh_keys::key::KeyPair;
use russh_sftp::protocol::{FileAttributes, Handle, OpenFlags, Status, StatusCode, Version};
use tokio::sync::Mutex;

use once_cell::sync::OnceCell;

use log::{error, info};
use simplelog::{Config as LogConfig, LevelFilter as LogLevelFilter, SimpleLogger, WriteLogger};

mod config;

// static global_server_config: ServerConfig;
static SERVER_CONFIG: OnceCell<ServerConfig> = OnceCell::new();

fn init_config() -> Result<(), anyhow::Error> {
    let args: Vec<String> = std::env::args().collect();
    let mut srv_cfg: ServerConfig;

    if args.len() < 2 {
        srv_cfg = ServerConfig::default();
    } else {
        srv_cfg = ServerConfig::try_from(&args[1])?;
    }

    match &srv_cfg.log_file_path {
        Some(s) => WriteLogger::init(
            LogLevelFilter::Info,
            LogConfig::default(),
            std::fs::File::create(&s).unwrap(),
        ),
        None => SimpleLogger::init(LogLevelFilter::Info, LogConfig::default()),
    }?;

    srv_cfg.sftp_base_dir = match srv_cfg.sftp_base_dir {
        Some(mut s) => match s.rfind('/') {
            Some(ind) => {
                if ind != s.len() - 1 {
                    s.push('/');
                }
                Some(s)
            }
            None => {
                s.push('/');
                Some(s)
            }
        },
        None => Some(String::from("./")),
    };

    SERVER_CONFIG
        .set(srv_cfg)
        .expect("server config already initialized");

    Ok(())
}

#[tokio::main]
async fn main() {
    init_config().unwrap();
    let srv_cfg = SERVER_CONFIG.get().unwrap();

    let config = russh::server::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        methods: russh::MethodSet::all(),
        keys: vec![KeyPair::generate_ed25519().unwrap()],
        ..Default::default()
    };

    let mut server = Server;

    info!("Server running on {}:{}", &srv_cfg.interface, &srv_cfg.port);
    server
        .run_on_address(
            Arc::new(config),
            (srv_cfg.interface.clone(), srv_cfg.port.parse().unwrap()),
        )
        .await
        .unwrap();
}

#[derive(Clone)]
struct Server;

impl russh::server::Server for Server {
    type Handler = SshSession;

    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self::Handler {
        SshSession::default()
    }
}

struct SshSession {
    clients: Arc<Mutex<HashMap<ChannelId, Channel<Msg>>>>,
}

impl Default for SshSession {
    fn default() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl SshSession {
    pub async fn take_channel(&mut self, channel_id: ChannelId) -> Channel<Msg> {
        let mut clients = self.clients.lock().await;
        clients.remove(&channel_id).unwrap()
    }
}

#[async_trait]
impl russh::server::Handler for SshSession {
    type Error = anyhow::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!("New client received on channel id {}", &channel.id());
        {
            let mut clients = self.clients.lock().await;
            clients.insert(channel.id(), channel);
        }
        Ok(true)
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!("Client on channel {} has closed.", channel);
        session.close(channel);
        Ok(())
    }

    async fn auth_publickey(
        &mut self,
        _: &str,
        pkey: &russh_keys::key::PublicKey,
    ) -> Result<russh::server::Auth, Self::Error> {
        let srv_cfg = SERVER_CONFIG.get().unwrap();
        let auth_file = srv_cfg.authorized_key_file.as_str();
        let file = std::fs::File::open(auth_file)?;
        let buf_reader = BufReader::new(file);

        for line in buf_reader.lines().map(|l| l.unwrap()) {
            let b64: Vec<&str> = line.split(' ').collect();
            if b64.len() < 3 {
                continue;
            }

            let key = russh_keys::parse_public_key_base64(&b64[1])?;

            if key.eq(pkey) {
                info!("Client authenticated with public key");
                return Ok(russh::server::Auth::Accept);
            }
        }

        error!("Client failed to authenticate with public key");
        Ok(russh::server::Auth::Reject {
            proceed_with_methods: None,
        })
    }

    async fn subsystem_request(
        &mut self,
        channel_id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if name == "sftp" {
            let channel = self.take_channel(channel_id).await;
            let sftp = SftpSession::default();
            session.channel_success(channel_id);
            russh_sftp::server::run(channel.into_stream(), sftp).await;
        } else {
            session.channel_failure(channel_id)
        }

        Ok(())
    }
}

#[derive(Default)]
struct SftpSession {
    version: Option<u32>,
    open_files: Arc<Mutex<HashMap<String, File>>>,
}

#[async_trait]
impl russh_sftp::server::Handler for SftpSession {
    type Error = StatusCode;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported
    }

    async fn init(
        &mut self,
        _version: u32,
        _extensions: HashMap<String, String>,
    ) -> Result<Version, Self::Error> {
        if self.version.is_some() {
            error!("duplicate SSH_FXP_VERSION packet");
            return Err(StatusCode::ConnectionLost);
        }

        Ok(Version::new())
    }

    async fn open(
        &mut self,
        id: u32,
        filename: String,
        pflags: OpenFlags,
        _attrs: FileAttributes,
    ) -> Result<Handle, Self::Error> {
        info!(
            "open called: id {} filename {} pflags {:?}",
            &id, &filename, &pflags
        );

        let srv_cfg = SERVER_CONFIG.get().unwrap();

        let mut full_path = srv_cfg.sftp_base_dir.as_ref().unwrap().clone();

        // only allows base directory to be accessed through sftp
        if let Some(ind) = filename.find('/') {
            if ind == 0 {
                error!(
                    "Client {} trying to access a disallowed directory: {}",
                    id, filename
                );
                return Err(StatusCode::PermissionDenied);
            }
        }
        full_path.push_str(&filename);

        // do we parse pflags or do we make it write only?
        info!("attempting to open {}", &full_path);
        let file = match File::options().write(true).create(true).open(&full_path) {
            Ok(f) => f,
            Err(e) => {
                error!("Error opening file {} {:?}", &full_path, e);
                return Err(StatusCode::Failure);
            }
        };

        {
            let mut mutex_opened_files = self.open_files.lock().await;
            if mutex_opened_files.contains_key(&full_path) {
                error!("Opened files map does not contain path {} ", full_path);
                return Err(StatusCode::Failure);
            } else {
                mutex_opened_files.insert(full_path.clone(), file);
            }
        }

        Ok(Handle {
            id,
            handle: full_path,
        })
    }

    async fn write(
        &mut self,
        id: u32,
        handle: String,
        _offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, Self::Error> {
        // need to acquire lock for this whole block
        let mut m_opened_files = self.open_files.lock().await;

        let f = match m_opened_files.get_mut(&handle) {
            Some(f) => f,
            None => return Err(StatusCode::NoSuchFile),
        };

        match f.write(&data) {
            Ok(_) => Ok(Status {
                id,
                status_code: StatusCode::Ok,
                error_message: String::from("Ok"),
                language_tag: String::from("en-US"),
            }),
            Err(_) => Err(StatusCode::Failure),
        }
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
        info!("call closed with id {} and handle {}", id, &handle);

        let mut _removed: Option<File> = None;
        {
            let mut mutex_opened_files = self.open_files.lock().await;
            _removed = mutex_opened_files.remove(&handle);
        }

        match _removed {
            Some(_) => {
                info!("file {} closed successfully", &handle);
                Ok(Status {
                    id,
                    status_code: StatusCode::Ok,
                    error_message: "Ok".to_string(),
                    language_tag: "en-US".to_string(),
                })
            }
            None => Err(StatusCode::NoSuchFile),
        }
    }

    async fn mkdir(
        &mut self,
        id: u32,
        path: String,
        _attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        let srv_cfg = SERVER_CONFIG.get().unwrap();
        let mut full_path = srv_cfg.sftp_base_dir.as_ref().unwrap().clone();

        // maybe fix with regex down the line
        let new_p = std::path::Path::new(&path);
        let empty = Path::new("");

        // new_p.parent().unwrap().as_os_str().to_str()
        if let Some(parent_path) = new_p.parent() {
            if parent_path != empty {
                error!(
                    "Client on id {} trying to access out of scope directory: {}",
                    id, path
                );
                return Err(StatusCode::PermissionDenied);
            }
        } else {
            error!(
                "Client on id {} trying to access malformed directory: {}",
                id, path
            );
            return Err(StatusCode::PermissionDenied);
        }

        if full_path.eq(srv_cfg.sftp_base_dir.as_ref().unwrap()) {
            full_path.push_str(&path);
        }

        info!(
            "Attempting to create dir {} for client on id {}",
            &full_path, id
        );

        match std::fs::create_dir_all(&full_path) {
            Ok(_) => Ok(Status {
                id,
                status_code: StatusCode::Ok,
                error_message: "Ok".to_string(),
                language_tag: "en-US".to_string(),
            }),
            Err(_) => Err(StatusCode::Failure),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_logger(srv_cfg: &ServerConfig) {
        match &srv_cfg.log_file_path {
            Some(s) => WriteLogger::init(
                LogLevelFilter::Info,
                LogConfig::default(),
                std::fs::File::create(s).unwrap(),
            ),
            None => SimpleLogger::init(LogLevelFilter::Info, LogConfig::default()),
        }
        .unwrap();
    }

    /// run with `cargo test -- --nocapture`
    #[test]
    fn test_get_config() {
        let result = config::ServerConfig::try_from("test.json").unwrap();
        cfg_logger(&result);

        let interface = &result.interface;
        let port = &result.port;
        let keyfile = &result.authorized_key_file;
        let log_file = &result.log_file_path;

        info!(
            "Set to run on {}:{}, with keyfile {} and logfile {:?}",
            interface, port, keyfile, log_file
        );
    }

    /// run with `cargo test -- --nocapture`
    #[test]
    fn test_default_config() {
        let result = config::ServerConfig::default();

        let interface = &result.interface;
        let port = &result.port;
        let keyfile = &result.authorized_key_file;

        info!(
            "Set to run on {}:{}, with keyfile {}",
            interface, port, keyfile
        );
    }
}
