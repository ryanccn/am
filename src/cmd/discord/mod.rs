// SPDX-FileCopyrightText: 2025 Ryan Cao <hello@ryanccn.dev>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::Duration;
use tokio::{signal, time};

use anstream::{eprintln, println};
use eyre::{Result, eyre};
use owo_colors::OwoColorize as _;

use crate::{
    music,
    rich_presence::{
        DiscordIpc, DiscordIpcClient, RichPresenceError,
        activity::{Activity, Assets, Button, Timestamps},
    },
};

pub mod agent;

#[derive(Debug, Clone)]
struct ActivityState {
    last_song_id: Option<String>,
    last_position: Option<f64>,
    is_idle: bool,
}

#[expect(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
async fn update_presence(client: &mut DiscordIpcClient, state: &mut ActivityState) -> Result<()> {
    if !music::is_running().await? {
        if !state.is_idle {
            println!("{} any songs", "Not playing".yellow());
            state.last_position = None;
            state.last_song_id = None;
            state.is_idle = true;
        }

        client.clear_activity().await?;
        return Ok(());
    }

    let initial_state = music::tell("get {player position, player state}").await?;
    let mut initial_state = initial_state.split(", ");

    let position = initial_state
        .next()
        .ok_or_else(|| eyre!("Could not obtain player position"))?;
    let player_state = initial_state
        .next()
        .ok_or_else(|| eyre!("Could not obtain player state"))?
        .parse::<music::PlayerState>()?;

    if player_state != music::PlayerState::Playing {
        if !state.is_idle {
            println!("{} any songs", "Not playing".yellow());
            state.last_position = None;
            state.last_song_id = None;
            state.is_idle = true;
        }

        client.clear_activity().await?;
        return Ok(());
    }

    let position = position.replace(",", ".").parse::<f64>()?;

    let track = music::get_current_track()
        .await?
        .ok_or_else(|| eyre!("Could not obtain track information"))?;

    let mut ongoing = false;

    if let Some(last_song_id) = &state.last_song_id
        && *last_song_id == track.id
        && let Some(last_position) = &state.last_position
        && last_position <= &position
    {
        ongoing = true;
    }

    if !ongoing {
        let metadata = match music::fetch_metadata(&track).await {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("failed to fetch metadata: {e:?}");
                None
            }
        };

        let now_ts = chrono::offset::Local::now().timestamp();
        let start_ts = (now_ts as f64) - position;
        let end_ts = (now_ts as f64) + track.duration - position;

        let activity_state = format!("{} · {}", &track.artist, &track.album);

        let mut activity = Activity::new().details(&track.name).state(&activity_state);

        if let Some(metadata) = &metadata {
            let mut activity_assets = Assets::new()
                .large_image(&metadata.album_artwork)
                .large_text(&track.name);

            if let Some(artist_artwork) = &metadata.artist_artwork {
                activity_assets = activity_assets
                    .small_image(artist_artwork)
                    .small_text(&track.artist);
            }

            activity = activity.assets(activity_assets);
        }

        activity = activity.timestamps(
            Timestamps::new()
                .start(start_ts.floor() as i64)
                .end(end_ts.ceil() as i64),
        );

        if let Some(metadata) = &metadata {
            activity = activity.buttons(vec![
                Button::new("Listen on Apple Music", &metadata.share_url)?,
                Button::new("View on SongLink", &metadata.song_link)?,
            ])?;
        }

        client.set_activity(activity).await?;

        println!(
            "{} {} {} {}",
            "Song updated".blue(),
            &track.name,
            "·".dimmed(),
            &track.artist,
        );

        state.last_position = Some(position);
        state.last_song_id = Some(track.id.clone());
        state.is_idle = false;
    }

    Ok(())
}

pub async fn discord() -> Result<()> {
    let mut client = DiscordIpcClient::new("861702238472241162");
    if client.connect().await.is_ok() {
        println!("{} to Discord", "Connected".green());
    }

    let mut state = ActivityState {
        last_position: None,
        last_song_id: None,
        is_idle: false,
    };

    let mut last_connect_failed = false;
    let mut intvl = time::interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            _ = intvl.tick() => {
                if let Err(err) = update_presence(&mut client, &mut state).await {
                    match err.downcast_ref::<RichPresenceError>() {
                        Some(RichPresenceError::CouldNotConnect | RichPresenceError::WriteSocketFailed) => {
                            if !last_connect_failed {
                                eprintln!("{} from Discord", "Disconnected".red());
                                last_connect_failed = true;
                            }
                        },
                        _ => {
                            eprintln!("{} {}", "Error".red(), err);
                        },
                    }
                } else if last_connect_failed {
                    last_connect_failed = false;
                    eprintln!("{} to Discord", "Connected".green());
                }
            }

            _ = signal::ctrl_c() => {
                break;
            }
        }
    }

    println!("{} Discord presence", "Shutting down".yellow());
    client.clear_activity().await?;
    client.close().await?;

    Ok(())
}
