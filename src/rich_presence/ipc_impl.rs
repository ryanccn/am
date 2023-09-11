use crate::rich_presence::DiscordIpc;

use owo_colors::OwoColorize;
use serde_json::json;

use std::{env::var, path::PathBuf};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

use anyhow::{anyhow, Result};
use async_trait::async_trait;

// Environment keys to search for the Discord pipe
const ENV_KEYS: [&str; 4] = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"];

#[allow(dead_code)]
/// A wrapper struct for the functionality contained in the
/// underlying [`DiscordIpc`](trait@DiscordIpc) trait.
pub struct DiscordIpcClient {
    /// Client ID of the IPC client.
    pub client_id: String,
    connected: bool,
    socket: Option<UnixStream>,
}

impl DiscordIpcClient {
    /// Creates a new `DiscordIpcClient`.
    ///
    /// # Examples
    /// ```
    /// let ipc_client = DiscordIpcClient::new("<some client id>")?;
    /// ```
    pub fn new(client_id: &str) -> Result<Self> {
        let client = Self {
            client_id: client_id.to_string(),
            connected: false,
            socket: None,
        };

        Ok(client)
    }

    fn get_pipe_pattern() -> PathBuf {
        let mut path = String::new();

        for key in &ENV_KEYS {
            match var(key) {
                Ok(val) => {
                    path = val;
                    break;
                }
                Err(_e) => continue,
            }
        }

        PathBuf::from(path)
    }
}

#[async_trait]
impl DiscordIpc for DiscordIpcClient {
    async fn connect_ipc(&mut self) -> Result<()> {
        for i in 0..10 {
            let path = DiscordIpcClient::get_pipe_pattern().join(format!("discord-ipc-{}", i));

            match UnixStream::connect(&path).await {
                Ok(socket) => {
                    self.socket = Some(socket);
                    return Ok(());
                }
                Err(_) => continue,
            }
        }

        Err(anyhow!("Couldn't connect to the Discord IPC socket"))
    }

    async fn write(&mut self, data: &[u8]) -> Result<()> {
        let socket = self
            .socket
            .as_mut()
            .ok_or_else(|| anyhow!("Client not connected"))?;

        if socket.write_all(data).await.is_err() {
            eprintln!("{} to Discord", "Reconnecting".yellow());
            self.connect_ipc().await?;
            self.send_handshake().await?;

            let socket = self
                .socket
                .as_mut()
                .ok_or_else(|| anyhow!("Client not connected"))?;
            socket.write_all(data).await?;
        };

        Ok(())
    }

    async fn read(&mut self, buffer: &mut [u8]) -> Result<()> {
        let socket = self
            .socket
            .as_mut()
            .ok_or_else(|| anyhow!("Client not connected"))?;

        if socket.read_exact(buffer).await.is_err() {
            eprintln!("{} to Discord", "Reconnecting".yellow());
            self.connect_ipc().await?;
            self.send_handshake().await?;

            let socket = self
                .socket
                .as_mut()
                .ok_or_else(|| anyhow!("Client not connected"))?;
            socket.read_exact(buffer).await?;
        };

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        let data = json!({});
        self.send(data, 2).await?;

        let socket = self.socket.as_mut().unwrap();

        socket.flush().await?;
        match socket.shutdown().await {
            Ok(()) => (),
            Err(_err) => (),
        };

        Ok(())
    }

    fn get_client_id(&self) -> &String {
        &self.client_id
    }
}
