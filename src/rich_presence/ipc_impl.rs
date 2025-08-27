// SPDX-FileCopyrightText: 2025 Ryan Cao <hello@ryanccn.dev>
// SPDX-FileCopyrightText: 2022 sardonicism-04
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::errors::RichPresenceError;
use crate::rich_presence::DiscordIpc;

use serde_json::json;

use std::{env, path::PathBuf};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

use async_trait::async_trait;

// Environment keys to search for the Discord pipe
const ENV_KEYS: [&str; 4] = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"];

/// A wrapper struct for the functionality contained in the
/// underlying [`DiscordIpc`](trait@DiscordIpc) trait.
pub struct DiscordIpcClient {
    /// Client ID of the IPC client.
    pub client_id: String,
    socket: Option<UnixStream>,
}

impl DiscordIpcClient {
    /// Creates a new `DiscordIpcClient`.
    ///
    /// # Examples
    /// ```
    /// let ipc_client = DiscordIpcClient::new("<some client id>")?;
    /// ```
    pub fn new(client_id: &str) -> Self {
        Self {
            client_id: client_id.to_string(),
            socket: None,
        }
    }

    fn get_pipe_pattern() -> Result<PathBuf, RichPresenceError> {
        for key in &ENV_KEYS {
            if let Ok(val) = env::var(key) {
                return Ok(PathBuf::from(val));
            }
        }

        Err(RichPresenceError::CouldNotConnect)
    }
}

#[async_trait]
impl DiscordIpc for DiscordIpcClient {
    async fn connect_ipc(&mut self) -> Result<(), RichPresenceError> {
        for i in 0..10 {
            let path = DiscordIpcClient::get_pipe_pattern()?.join(format!("discord-ipc-{i}"));

            if let Ok(socket) = UnixStream::connect(&path).await {
                self.socket = Some(socket);
                return Ok(());
            }
        }

        Err(RichPresenceError::CouldNotConnect)
    }

    async fn write_once(&mut self, data: &[u8]) -> Result<(), RichPresenceError> {
        let socket = self
            .socket
            .as_mut()
            .ok_or_else(|| RichPresenceError::CouldNotConnect)?;

        socket
            .write_all(data)
            .await
            .map_err(|_| RichPresenceError::WriteSocketFailed)?;

        Ok(())
    }

    async fn write(&mut self, data: &[u8]) -> Result<(), RichPresenceError> {
        match self.write_once(data).await {
            Err(RichPresenceError::CouldNotConnect | RichPresenceError::WriteSocketFailed) => {
                self.connect().await?;
                self.write_once(data).await?;
                Ok(())
            }
            rest => rest,
        }?;

        Ok(())
    }

    async fn read_once(&mut self, buffer: &mut [u8]) -> Result<(), RichPresenceError> {
        let socket = self
            .socket
            .as_mut()
            .ok_or_else(|| RichPresenceError::CouldNotConnect)?;

        socket
            .read_exact(buffer)
            .await
            .map_err(|_| RichPresenceError::ReadSocketFailed)?;

        Ok(())
    }

    async fn read(&mut self, buffer: &mut [u8]) -> Result<(), RichPresenceError> {
        match self.read_once(buffer).await {
            Err(RichPresenceError::CouldNotConnect | RichPresenceError::WriteSocketFailed) => {
                self.connect().await?;
                self.read_once(buffer).await?;
                Ok(())
            }
            rest => rest,
        }?;

        Ok(())
    }

    async fn close(&mut self) -> Result<(), RichPresenceError> {
        let data = json!({});
        self.send(data, 2).await?;

        let socket = self.socket.as_mut().unwrap();

        socket
            .flush()
            .await
            .map_err(|_| RichPresenceError::FlushSocketFailed)?;
        match socket.shutdown().await {
            Ok(()) => (),
            Err(_err) => (),
        }

        Ok(())
    }

    fn get_client_id(&self) -> &String {
        &self.client_id
    }
}
