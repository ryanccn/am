// SPDX-FileCopyrightText: 2025 Ryan Cao <hello@ryanccn.dev>
// SPDX-FileCopyrightText: 2022 sardonicism-04
//
// SPDX-License-Identifier: GPL-3.0-or-later

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RichPresenceError {
    #[error("Could not connect to IPC socket")]
    CouldNotConnect,
    #[error("Received invalid packet")]
    RecvInvalidPacket,
    #[error("Failed to write to socket")]
    WriteSocketFailed,
    #[error("Failed to read from socket")]
    ReadSocketFailed,
    #[error("Failed to flush socket")]
    FlushSocketFailed,

    #[error("Invalid value when creating button")]
    ButtonCreateInvalidValue,
    #[error("Too many ({0}) buttons provided to activity")]
    TooManyButtons(usize),
}
