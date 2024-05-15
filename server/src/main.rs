use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use russh::server::{Msg, Server as _, Session};
use russh::*;
use russh_keys::*;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    let port = 2222;

    let config = russh::server::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        methods: MethodSet::all(),
        keys: vec![russh_keys::key::KeyPair::generate_ed25519().unwrap()],
        ..Default::default()
    };

    let config = Arc::new(config);

    let mut sh = Server {
        clients: Arc::new(Mutex::new(HashMap::new())),
        id: 0,
    };

    println!("Running on port {}", port);
    sh.run_on_address(config, ("127.0.0.1", port))
        .await
        .unwrap();
}

#[derive(Clone)]
struct Server {
    clients: Arc<Mutex<HashMap<(usize, ChannelId), russh::server::Handle>>>,
    id: usize,
}

impl Server {
    async fn post(&mut self, data: CryptoVec) {
        let mut clients = self.clients.lock().await;
        for ((id, channel), ref mut s) in clients.iter_mut() {
            if *id != self.id {
                let _ = s.data(*channel, data.clone()).await;
            }
        }
    }
}

impl server::Server for Server {
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }
}

#[async_trait]
impl server::Handler for Server {
    type Error = anyhow::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        {
            let mut clients = self.clients.lock().await;
            clients.insert((self.id, channel.id()), session.handle());
        }
        println!("New client received on channel id {}", channel.id());
        Ok(true)
    }

    async fn channel_close(
        &mut self,
        channel: ChannelId,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        println!("Client on channel {} has closed.", channel);

        Ok(())
    }

    // async fn auth_publickey(
    //     &mut self,
    //     user: &str,
    //     public_key: &key::PublicKey,
    // ) -> Result<server::Auth, Self::Error> {
    //     println!("credentials: {}, {:?}", user, public_key);
    //     Ok(server::Auth::Accept)
    // }

    async fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &key::PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        println!("credentials: {}, {:?}", user, public_key);
        match public_key {
            key::PublicKey::Ed25519(ref key) => println!("Received Ed25519 key: {:?}", key),
            key::PublicKey::RSA { ref key, ref hash } => println!("Received RSA key: {:?}", hash),
            _ => println!("Received other key type"),
        }
        Ok(server::Auth::Accept)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        // let data = CryptoVec::from(format!("Got data: {}\r\n", String::from_utf8_lossy(data)));
        // self.post(data.clone()).await;
        // session.data(channel, data);
        println!(
            "Got data from channel id {}: {}",
            channel,
            String::from_utf8_lossy(data)
        );
        Ok(())
    }
}
