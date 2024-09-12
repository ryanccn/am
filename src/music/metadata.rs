use std::sync::{LazyLock, OnceLock};

use regex::Regex;
use reqwest::Client;

use color_eyre::eyre::{eyre, Result};

use super::Track;

static TOKEN_CACHE: OnceLock<String> = OnceLock::new();

static USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36";

static BUNDLE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<script type="module" crossorigin src="([a-zA-Z0-9.\-/]+)"></script>"#).unwrap()
});

static TOKEN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\w+="([A-Za-z0-9-_]*\.[A-Za-z0-9-_]*\.[A-Za-z0-9-_]*)",\w+="x-apple-jingle-correlation-key""#).unwrap()
});

#[derive(Debug)]
pub struct Metadata {
    pub album_artwork: String,
    pub artist_artwork: Option<String>,
    pub share_url: String,
    pub song_link: String,
}

async fn fetch_token(client: &Client) -> Result<String> {
    if let Some(token) = TOKEN_CACHE.get() {
        return Ok(token.to_owned());
    }

    let html = client
        .get("https://music.apple.com/")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let bundle_path = &BUNDLE_REGEX
        .captures(&html)
        .ok_or_else(|| eyre!("could not obtain bundle for API token"))?[1];

    let mut bundle_url = "https://music.apple.com/".parse::<reqwest::Url>()?;
    bundle_url.set_path(bundle_path);

    let bundle = client
        .get(bundle_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let token = &TOKEN_REGEX
        .captures(&bundle)
        .ok_or_else(|| eyre!("could not find API token in bundle"))?[1];

    TOKEN_CACHE.set(token.to_owned()).unwrap();

    Ok(token.to_owned())
}

pub async fn get_metadata(client: &Client, track: &Track) -> Result<Metadata> {
    let token = fetch_token(client).await?;
    let song_key = track.name.clone() + " " + &track.album + " " + &track.artist;

    let mut api_url =
        "https://amp-api-edge.music.apple.com/v1/catalog/us/search".parse::<reqwest::Url>()?;
    api_url
        .query_pairs_mut()
        .append_pair("platform", "web")
        .append_pair("l", "en-US")
        .append_pair("limit", "1")
        .append_pair("with", "serverBubbles")
        .append_pair("types", "songs")
        .append_pair("term", &song_key)
        .append_pair("include[songs]", "artists");

    let data: super::models::AppleMusicData = client
        .get(api_url)
        .bearer_auth(&token)
        .header("accept", "*/*")
        .header("accept-language", "en-US,en;q=0.9")
        .header("user-agent", USER_AGENT)
        .header("origin", "https://music.apple.com")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let track_data = data
        .results
        .song
        .data
        .first()
        .ok_or_else(|| eyre!("could not find track metadata"))?;

    let album_artwork = track_data
        .attributes
        .artwork
        .url
        .replace("{w}x{h}", "512x512");

    let artist_artwork = track_data
        .relationships
        .artists
        .data
        .first()
        .map(|data| data.attributes.artwork.url.replace("{w}x{h}", "512x512"));

    Ok(Metadata {
        album_artwork,
        artist_artwork,
        share_url: track_data.attributes.url.clone(),
        song_link: format!("https://song.link/i/{}", track_data.id),
    })
}
