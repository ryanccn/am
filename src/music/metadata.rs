use reqwest::Client;
use serde::{Deserialize, Serialize};

use anyhow::{anyhow, Result};

use super::Track;

pub struct Metadata {
    pub id: String,
    pub album_artwork: String,
    pub artist_artwork: Option<String>,
    pub share_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArtworkResult {
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArtistAttributes {
    artwork: ArtworkResult,
}

#[derive(Debug, Serialize, Deserialize)]
struct SongAttributes {
    url: String,
    artwork: ArtworkResult,
}

#[derive(Debug, Serialize, Deserialize)]
struct DataResult<T> {
    id: String,
    attributes: T,
}

#[derive(Debug, Serialize, Deserialize)]
struct WrappedDataResult<T> {
    data: Vec<DataResult<T>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArtistMetadataResult {
    artists: WrappedDataResult<ArtistAttributes>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SongMetadataResult {
    songs: WrappedDataResult<SongAttributes>,
}

fn make_api_url(type_: &str, term: &str) -> Result<reqwest::Url> {
    let mut ret = "https://tools.applemediaservices.com/api/apple-media/music/US/search.json"
        .parse::<reqwest::Url>()?;
    ret.query_pairs_mut()
        .append_pair("types", type_)
        .append_pair("limit", "1")
        .append_pair("term", term);

    Ok(ret)
}

pub async fn get_metadata(client: &Client, track: &Track) -> Result<Metadata> {
    let song_key = track.artist.clone() + " " + &track.album + " " + &track.name;
    let artist_key = track
        .artist
        .split(&[',', '&'][..])
        .next()
        .ok_or_else(|| anyhow!("Could not obtain artist to query with"))?;

    let mut artist_artwork: Option<String> = None;

    let (song_resp, artist_resp) = tokio::try_join!(
        client.get(make_api_url("songs", &song_key)?).send(),
        client.get(make_api_url("artists", &artist_key)?).send()
    )?;

    let (song_resp_data, artist_resp_data): (SongMetadataResult, ArtistMetadataResult) =
        tokio::try_join!(song_resp.json(), artist_resp.json())?;

    let song_data = song_resp_data
        .songs
        .data
        .first()
        .ok_or_else(|| anyhow!("Could not obtain song metadata"))?;

    let id: String = song_data.id.clone();
    let album_artwork: String = song_data
        .attributes
        .artwork
        .url
        .clone()
        .replace("{w}", "512")
        .replace("{h}", "512");
    let share_url: String = song_data.attributes.url.clone();

    let artist_data = artist_resp_data.artists.data.first();

    if let Some(artist_data) = artist_data {
        artist_artwork = Some(
            artist_data
                .attributes
                .artwork
                .url
                .clone()
                .replace("{w}", "512")
                .replace("{h}", "512"),
        );
    }

    Ok(Metadata {
        id,
        album_artwork,
        artist_artwork,
        share_url,
    })
}
