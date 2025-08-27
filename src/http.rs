// SPDX-FileCopyrightText: 2025 Ryan Cao <hello@ryanccn.dev>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::LazyLock;

use reqwest::Client;

pub static HTTP: LazyLock<Client> =
    LazyLock::new(|| Client::builder().https_only(true).build().unwrap());
