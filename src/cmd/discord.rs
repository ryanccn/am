use std::time::Duration;

use owo_colors::OwoColorize;

use anyhow::{anyhow, Result};
use tokio::signal::ctrl_c;

use crate::{
    music,
    rich_presence::{
        activity::{Activity, Assets, Button, Timestamps},
        DiscordIpc, DiscordIpcClient,
    },
};

#[derive(Debug, Clone)]
struct Song {
    id: i64,
    name: String,
    artist: String,
    album: String,
    duration: f64,
    album_artwork: String,
    artist_artwork: Option<String>,
    share_url: String,
    share_id: String,
}

#[derive(Debug, Clone)]
struct SongWithProgress {
    song: Option<Song>,
    position: Option<f64>,
}

#[derive(Debug, Clone)]
struct ActivityConnection {
    last_song_id: Option<i64>,
    last_position: Option<f64>,
    is_idle: bool,
}

async fn get_now_playing() -> Result<SongWithProgress> {
    let initial_state =
        music::tell("get {database id} of current track & {player position, player state}").await?;

    if initial_state.is_empty() {
        return Ok(SongWithProgress {
            song: None,
            position: None,
        });
    }

    let mut initial_state = initial_state.split(", ");

    let song_id = initial_state
        .next()
        .ok_or_else(|| anyhow!("Could not obtain song ID"))?;
    let position = initial_state
        .next()
        .ok_or_else(|| anyhow!("Could not obtain player position"))?;

    let state = initial_state
        .next()
        .ok_or_else(|| anyhow!("Could not obtain player state"))?;

    if state != "playing" {
        return Ok(SongWithProgress {
            song: None,
            position: None,
        });
    }

    let song_id = song_id.parse::<i64>()?;
    let position = position.parse::<f64>()?;

    let (name, album, artist, duration_str) = tokio::try_join!(
        music::tell("get {name} of current track"),
        music::tell("get {album} of current track"),
        music::tell("get {artist} of current track"),
        music::tell("get {duration} of current track")
    )?;
    let duration = duration_str.parse::<f64>()?;

    let client = reqwest::Client::new();

    let metadata =
        music::get_metadata(&client, artist.clone(), album.clone(), name.clone()).await?;

    Ok(SongWithProgress {
        song: Some({
            Song {
                id: song_id,
                name,
                artist,
                album,
                duration,
                album_artwork: metadata.album_artwork,
                artist_artwork: metadata.artist_artwork,
                share_url: metadata.share_url,
                share_id: metadata.id,
            }
        }),
        position: Some(position),
    })
}

async fn update_presence(
    client: &mut DiscordIpcClient,
    activity: &mut ActivityConnection,
) -> Result<()> {
    let now: SongWithProgress = get_now_playing().await?;

    let mut ongoing = false;

    if let Some(song) = now.song {
        if let Some(last_song_id) = activity.last_song_id {
            if last_song_id == song.id {
                if let Some(last_position) = activity.last_position {
                    if let Some(now_position) = now.position {
                        if last_position <= now_position {
                            ongoing = true;
                        }
                    }
                }
            }
        }

        if !ongoing {
            let position = now
                .position
                .ok_or_else(|| anyhow!("Could not obtain position data from result"))?;

            activity.last_position = Some(position);
            activity.last_song_id = Some(song.id);
            activity.is_idle = false;

            println!(
                "{} {} - {}",
                "Song updated".blue(),
                &song.name,
                &song.artist
            );

            let now_ts = chrono::offset::Local::now().timestamp();
            let start_ts = (now_ts as f64) - position;
            let end_ts = (now_ts as f64) + song.duration - position;

            let activity_state = format!("{} · {}", &song.artist, &song.album);

            let mut activity = Activity::new().details(&song.name).state(&activity_state);

            let mut activity_assets = Assets::new();

            activity_assets = activity_assets
                .large_image(&song.album_artwork)
                .large_text(&song.name);

            if let Some(artist_artwork) = song.artist_artwork {
                activity_assets = activity_assets
                    .small_image(&artist_artwork)
                    .small_text(&song.artist);
            }

            activity = activity.assets(activity_assets);
            activity = activity.timestamps(
                Timestamps::new()
                    .start(start_ts.floor() as i64)
                    .end(end_ts.ceil() as i64),
            );

            activity = activity.buttons(vec![
                Button::new("Listen on Apple Music", &song.share_url),
                Button::new(
                    "View on SongLink",
                    &format!("https://song.link/i/{}", song.share_id),
                ),
            ]);

            client.set_activity(activity).await?;
        }
    } else if !activity.is_idle {
        println!("{} any songs", "Not playing".yellow());
        activity.last_position = None;
        activity.last_song_id = None;
        activity.is_idle = true;
    }

    Ok(())
}

pub async fn discord() -> Result<()> {
    let mut client = DiscordIpcClient::new("861702238472241162")?;
    client.connect().await?;

    println!("{} to Discord", "Connected".green());

    let mut activity = ActivityConnection {
        last_position: None,
        last_song_id: None,
        is_idle: false,
    };

    let mut intvl = tokio::time::interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            _ = intvl.tick() => {
                if let Err(err) = update_presence(&mut client, &mut activity).await {
                    eprintln!("{} {}", "Error".red(), err);
                }
            }
            _ = ctrl_c() => {
                break;
            }
        }
    }

    client.close().await?;
    println!("{} Discord presence", "Shutting down".yellow());

    Ok(())
}
