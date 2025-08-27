// SPDX-FileCopyrightText: 2025 Ryan Cao <hello@ryanccn.dev>
// SPDX-FileCopyrightText: 2022 sardonicism-04
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::rich_presence::{
    activity::Activity,
    errors::RichPresenceError,
    pack_unpack::{pack, unpack},
};
use async_trait::async_trait;
use serde_json::{Value, json};

use uuid::Uuid;

/// A client that connects to and communicates with the Discord IPC.
///
/// Implemented via the [`DiscordIpcClient`](struct@crate::rich_presence::DiscordIpcClient) struct.
#[expect(clippy::cast_possible_truncation)]
#[async_trait]
pub trait DiscordIpc {
    /// Connects the client to the Discord IPC.
    ///
    /// This method attempts to first establish a connection,
    /// and then sends a handshake.
    ///
    /// # Errors
    ///
    /// Returns an `Err` variant if the client
    /// fails to connect to the socket, or if it fails to
    /// send a handshake.
    ///
    /// # Examples
    /// ```
    /// let mut client = crate::rich_presence::new_client("<some client id>")?;
    /// client.connect()?;
    /// ```
    async fn connect(&mut self) -> Result<(), RichPresenceError> {
        self.connect_ipc().await?;
        self.send_handshake().await?;

        Ok(())
    }

    #[doc(hidden)]
    fn get_client_id(&self) -> &String;

    #[doc(hidden)]
    async fn connect_ipc(&mut self) -> Result<(), RichPresenceError>;

    /// Handshakes the Discord IPC.
    ///
    /// This method sends the handshake signal to the IPC.
    /// It is usually not called manually, as it is automatically
    /// called by [`connect`] and/or [`reconnect`].
    ///
    /// [`connect`]: #method.connect
    /// [`reconnect`]: #method.reconnect
    ///
    /// # Errors
    ///
    /// Returns an `Err` variant if sending the handshake failed.
    async fn send_handshake(&mut self) -> Result<(), RichPresenceError> {
        self.send(
            json!({
                "v": 1,
                "client_id": self.get_client_id()
            }),
            0,
        )
        .await?;

        // TODO: Return an Err if the handshake is rejected
        self.recv().await?;

        Ok(())
    }

    /// Sends JSON data to the Discord IPC.
    ///
    /// This method takes data (`serde_json::Value`) and
    /// an opcode as its parameters.
    ///
    /// # Errors
    /// Returns an `Err` variant if writing to the socket failed
    ///
    /// # Examples
    /// ```
    /// let payload = serde_json::json!({ "field": "value" });
    /// client.send(payload, 0).await?;
    /// ```
    async fn send(&mut self, data: Value, opcode: u8) -> Result<(), RichPresenceError> {
        let data_string = data.to_string();
        let header = pack(opcode.into(), data_string.len() as u32);

        self.write(&header).await?;
        self.write(data_string.as_bytes()).await?;

        Ok(())
    }

    #[doc(hidden)]
    async fn write_once(&mut self, data: &[u8]) -> Result<(), RichPresenceError>;
    #[doc(hidden)]
    async fn write(&mut self, data: &[u8]) -> Result<(), RichPresenceError>;

    /// Receives an opcode and JSON data from the Discord IPC.
    ///
    /// This method returns any data received from the IPC.
    /// It returns a tuple containing the opcode, and the JSON data.
    ///
    /// # Errors
    /// Returns an `Err` variant if reading the socket was
    /// unsuccessful.
    ///
    /// # Examples
    /// ```
    /// client.connect_ipc().await?;
    /// client.send_handshake().await?;
    ///
    /// println!("{:?}", client.recv().await?);
    /// ```
    async fn recv(&mut self) -> Result<(u32, Value), RichPresenceError> {
        let mut header = [0; 8];

        self.read(&mut header).await?;
        let (op, length) = unpack(&header)?;

        let mut data = vec![0u8; length as usize];
        self.read(&mut data).await?;

        let response =
            String::from_utf8(data.clone()).map_err(|_| RichPresenceError::RecvInvalidPacket)?;
        let json_data = serde_json::from_str::<Value>(&response)
            .map_err(|_| RichPresenceError::RecvInvalidPacket)?;

        Ok((op, json_data))
    }

    #[doc(hidden)]
    async fn read_once(&mut self, buffer: &mut [u8]) -> Result<(), RichPresenceError>;
    #[doc(hidden)]
    async fn read(&mut self, buffer: &mut [u8]) -> Result<(), RichPresenceError>;

    /// Sets a Discord activity.
    ///
    /// This method is an abstraction of [`send`],
    /// wrapping it such that only an activity payload
    /// is required.
    ///
    /// [`send`]: #method.send
    ///
    /// # Errors
    /// Returns an `Err` variant if sending the payload failed.
    async fn set_activity(&mut self, activity_payload: Activity) -> Result<(), RichPresenceError> {
        let data = json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": std::process::id(),
                "activity": activity_payload
            },
            "nonce": Uuid::new_v4().to_string()
        });
        self.send(data, 1).await?;

        Ok(())
    }

    /// Works the same as as [`set_activity`] but clears activity instead.
    ///
    /// [`set_activity`]: #method.set_activity
    ///
    /// # Errors
    /// Returns an `Err` variant if sending the payload failed.
    async fn clear_activity(&mut self) -> Result<(), RichPresenceError> {
        let data = json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": std::process::id(),
                "activity": None::<()>
            },
            "nonce": Uuid::new_v4().to_string()
        });

        self.send(data, 1).await?;

        Ok(())
    }

    /// Closes the Discord IPC connection. Implementation is dependent on platform.
    async fn close(&mut self) -> Result<(), RichPresenceError>;
}
