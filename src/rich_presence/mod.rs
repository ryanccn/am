// SPDX-FileCopyrightText: 2025 Ryan Cao <hello@ryanccn.dev>
// SPDX-FileCopyrightText: 2022 sardonicism-04
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This library provides easy access to the Discord IPC.
//!
//! It provides implementations for both Unix and Windows
//! operating systems, with both implementations using the
//! same API. Thus, this crate can be used in a platform-agnostic
//! manner.
//!
//! # Hello world
//! ```
//! use crate::rich_presence::{activity, DiscordIpc, DiscordIpcClient};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut client = DiscordIpcClient::new("<some client id>")?;
//!     client.connect()?;
//!
//!     let payload = activity::Activity::new().state("Hello world!");
//!     client.set_activity(payload)?;
//! }
//! ```

mod errors;
pub use errors::RichPresenceError;

mod ipc_trait;
mod pack_unpack;

pub use ipc_trait::*;
pub mod activity;

mod ipc_impl;
pub use ipc_impl::DiscordIpcClient;
