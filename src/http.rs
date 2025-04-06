use std::sync::LazyLock;

use reqwest::Client;

pub static HTTP: LazyLock<Client> =
    LazyLock::new(|| Client::builder().https_only(true).build().unwrap());
