use crate::{format, music};

use anyhow::{anyhow, Result};
use crossterm::{cursor, execute, terminal};
use owo_colors::OwoColorize;

use std::{io::stdout, sync::Arc, time::Duration};
use tokio::{signal::ctrl_c, sync::Mutex};

#[derive(Debug, Clone)]
struct PlaybackState {
    state: String,
    position: Option<f32>,
    track: Option<Track>,
    playlist: Option<Playlist>,
}

#[derive(Debug, Clone)]
struct Track {
    id: String,
    name: String,
    album: String,
    artist: String,
    duration: f32,
}

#[derive(Debug, Clone)]
struct Playlist {
    name: String,
    duration: i32,
}

async fn update_state(data: &Arc<Mutex<PlaybackState>>) -> Result<()> {
    let mut data = data.lock().await;

    let player_state = music::tell("player state").await?;
    data.state = player_state.clone();

    if player_state == "stopped" {
        println!("Playback is {}", "stopped".red());
    } else {
        let (track_id, player_position, playlist_name) = tokio::try_join!(
            music::tell("get {database id} of current track"),
            music::tell("player position"),
            music::tell("get {name} of current playlist"),
        )?;

        let player_position = player_position.parse::<f32>().ok();
        data.position = player_position;

        let mut retrieve_track_data = true;

        if let Some(track) = &data.track {
            if track_id == track.id {
                retrieve_track_data = false;
            }
        }

        if retrieve_track_data {
            let (track_name, track_album, track_artist, track_duration_str) = tokio::try_join!(
                music::tell("get {name} of current track"),
                music::tell("get {album} of current track"),
                music::tell("get {artist} of current track"),
                music::tell("get {duration} of current track")
            )?;

            let track_duration = track_duration_str.parse::<f32>()?;

            data.track = Some(Track {
                id: track_id,
                name: track_name,
                album: track_album,
                artist: track_artist,
                duration: track_duration,
            });
        }

        let mut playlist: Option<Playlist> = None;

        if !playlist_name.is_empty() {
            let playlist_duration = music::tell("get {duration} of current playlist")
                .await?
                .parse::<i32>()?;

            playlist = Some(Playlist {
                name: playlist_name.to_string(),
                duration: playlist_duration,
            });
        }

        data.playlist = playlist;
    }

    Ok(())
}

async fn update_display(data: &Arc<Mutex<PlaybackState>>, watch: bool) -> Result<()> {
    let data = data.lock().await;

    let position = data
        .position
        .clone()
        .ok_or_else(|| anyhow!("Could not obtain position from shared playback state"))?;
    let track = data
        .track
        .clone()
        .ok_or_else(|| anyhow!("Could not obtain track data from shared playback state"))?;

    if watch {
        execute!(
            stdout(),
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All)
        )?;
    }

    println!("{}", track.name.bold());
    println!(
        "{} {}/{}",
        format::format_player_state(&data.state)?,
        format::format_duration(&position, false),
        format::format_duration(&track.duration, true),
    );
    println!("{} · {}", track.artist.blue(), track.album.magenta());

    if let Some(playlist) = &data.playlist {
        println!(
            "{}",
            format!(
                "Playlist: {} ({})",
                playlist.name,
                format::format_playlist_duration(&playlist.duration)
            )
            .dimmed()
        );
    } else {
        println!("{}", "No playlist".dimmed());
    }

    Ok(())
}

pub async fn now(watch: bool) -> Result<()> {
    if watch {
        execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;
    }

    let shared_data = Arc::new(Mutex::new(PlaybackState {
        state: "stopped".into(),
        position: None,
        playlist: None,
        track: None,
    }));

    if watch {
        let shared_data_state_update = shared_data.clone();

        let shared_stop = Arc::new(Mutex::new(false));
        let shared_stop_state_update = shared_stop.clone();
        let shared_stop_ctrlc = shared_stop.clone();

        let state_task = tokio::spawn(async move {
            let mut intvl = tokio::time::interval(Duration::from_millis(250));

            loop {
                if let Err(err) = update_state(&shared_data_state_update).await {
                    eprintln!("{}", err);
                };

                if shared_stop_state_update.lock().await.to_owned() {
                    break;
                };

                intvl.tick().await;
            }
        });

        let display_task = tokio::spawn(async move {
            let mut intvl = tokio::time::interval(Duration::from_millis(250));

            loop {
                if let Err(err) = update_display(&shared_data, watch).await {
                    eprintln!("{}", err);
                };

                if shared_stop.lock().await.to_owned() {
                    break;
                };

                intvl.tick().await;
            }
        });

        let ctrlc_task = tokio::spawn(async move {
            if ctrl_c().await.is_ok() {
                let mut stop = shared_stop_ctrlc.lock().await;
                *stop = true;
            }
        });

        tokio::try_join!(state_task, display_task, ctrlc_task)?;
    } else {
        update_state(&shared_data).await?;
        update_display(&shared_data, watch).await?;
    }

    if watch {
        execute!(stdout(), terminal::LeaveAlternateScreen, cursor::Show)?;
    }

    Ok(())
}
