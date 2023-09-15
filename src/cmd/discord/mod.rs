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

pub mod agent;

#[derive(Debug, Clone)]
struct ActivityConnection {
    last_song_id: Option<String>,
    last_position: Option<f64>,
    is_idle: bool,
}

async fn update_presence(
    client: &mut DiscordIpcClient,
    http_client: &reqwest::Client,
    activity: &mut ActivityConnection,
) -> Result<()> {
    if !music::is_running().await? {
        if !activity.is_idle {
            println!("{} any songs", "Not playing".yellow());
            activity.last_position = None;
            activity.last_song_id = None;
            activity.is_idle = true;
        }

        return Ok(());
    };

    let initial_state = music::tell("get {player position, player state}").await?;

    let mut initial_state = initial_state.split(", ");

    let position = initial_state
        .next()
        .ok_or_else(|| anyhow!("Could not obtain player position"))?;
    let state = initial_state
        .next()
        .ok_or_else(|| anyhow!("Could not obtain player state"))?;

    if state != "playing" {
        if !activity.is_idle {
            println!("{} any songs", "Not playing".yellow());
            activity.last_position = None;
            activity.last_song_id = None;
            activity.is_idle = true;
        }

        return Ok(());
    }

    let position = position.parse::<f64>()?;

    let track = music::get_current_track()
        .await?
        .ok_or_else(|| anyhow!("Could not obtain track information"))?;

    let mut ongoing = false;

    if let Some(last_song_id) = &activity.last_song_id {
        if *last_song_id == track.id {
            if let Some(last_position) = &activity.last_position {
                if last_position <= &position {
                    ongoing = true;
                }
            }
        }
    }

    if !ongoing {
        activity.last_position = Some(position);
        activity.last_song_id = Some(track.id.clone());
        activity.is_idle = false;

        println!(
            "{} {} - {}",
            "Song updated".blue(),
            &track.name,
            &track.artist
        );

        let metadata = music::get_metadata(&http_client, &track).await?;

        let now_ts = chrono::offset::Local::now().timestamp();
        let start_ts = (now_ts as f64) - position;
        let end_ts = (now_ts as f64) + track.duration - position;

        let activity_state = format!("{} · {}", &track.artist, &track.album);

        let mut activity = Activity::new().details(&track.name).state(&activity_state);

        let mut activity_assets = Assets::new()
            .large_image(&metadata.album_artwork)
            .large_text(&track.name);

        if let Some(artist_artwork) = metadata.artist_artwork {
            activity_assets = activity_assets
                .small_image(&artist_artwork)
                .small_text(&track.artist);
        }

        activity = activity.assets(activity_assets);
        activity = activity.timestamps(
            Timestamps::new()
                .start(start_ts.floor() as i64)
                .end(end_ts.ceil() as i64),
        );

        activity = activity.buttons(vec![
            Button::new("Listen on Apple Music", &metadata.share_url),
            Button::new(
                "View on SongLink",
                &format!("https://song.link/i/{}", track.id),
            ),
        ]);

        client.set_activity(activity).await?;
    };

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

    let http_client = reqwest::Client::new();

    let mut intvl = tokio::time::interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            _ = intvl.tick() => {
                if let Err(err) = update_presence(&mut client,& http_client, &mut activity).await {
                    eprintln!("{} {}", "Error".red(), err);
                }
            }
            _ = ctrl_c() => {
                break;
            }
        }
    }

    client.clear_activity().await?;
    client.close().await?;
    println!("{} Discord presence", "Shutting down".yellow());

    Ok(())
}
