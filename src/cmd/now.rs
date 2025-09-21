// SPDX-FileCopyrightText: 2025 Ryan Cao <hello@ryanccn.dev>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    format,
    music::{self, PlayerState, Playlist, Track},
};

use anstream::println;
use clap::Parser;
use crossterm::{cursor, execute, terminal};
use eyre::{Result, eyre};
use owo_colors::OwoColorize as _;

use std::{
    io::{Write as _, stdout},
    time::{Duration, Instant},
};
use tokio::{
    sync::{mpsc, watch},
    task,
};

#[derive(Parser, Debug)]
pub struct NowOptions {
    /// Show an keyboard-interactive, full-screen terminal UI
    #[arg(short, long)]
    pub watch: bool,

    /// Disable Nerd Font symbols
    #[arg(long)]
    pub no_nerd_fonts: bool,

    /// Playback progress bar width
    #[arg(long)]
    pub bar_width: Option<i32>,
}

#[derive(Debug, Clone)]
struct PlaybackState {
    state: PlayerState,
    position: Option<f64>,
    track: Option<Track>,
    playlist: Option<Playlist>,
    mystery_counter: i64,
}

#[derive(Debug)]
enum PlaybackStateDelta {
    State(PlayerState),
    Position(Option<f64>),
    PositionTick,
    TrackIDRequestMoreInfo(String),
    Track(Option<Track>),
    Playlist(Option<Playlist>),
    Render,
}

async fn update_state(
    tx: &mpsc::Sender<PlaybackStateDelta>,
    rx_request_track: &mut mpsc::Receiver<bool>,
) -> Result<()> {
    let player_state = music::tell("player state").await?.parse::<PlayerState>()?;

    tx.send(PlaybackStateDelta::State(player_state)).await?;

    if player_state != PlayerState::Stopped {
        let time_start = Instant::now();

        let data = music::tell_raw(&[
            r#"set output to """#,
            r#"tell application "Music""#,
            r"set track_id to database id of current track",
            r"set player_position to player position",
            r#"set output to "" & track_id & "\n" & player_position"#,
            r"end tell",
            r"return output",
        ])
        .await?;

        let time_latency = time_start.elapsed().as_secs_f64();

        let mut data = data.split('\n');

        let track_id = data
            .next()
            .ok_or_else(|| eyre!("Could not obtain track ID"))?
            .to_owned();
        let player_position = data
            .next()
            .ok_or_else(|| eyre!("Could not obtain player position"))?
            .to_owned();

        let playlist_name = music::tell("name of current playlist")
            .await
            .ok()
            .map(|s| s.trim().to_owned());

        let player_position = player_position
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .map(|p| p + time_latency);

        tx.send(PlaybackStateDelta::Position(player_position))
            .await?;

        tx.send(PlaybackStateDelta::TrackIDRequestMoreInfo(track_id.clone()))
            .await?;
        let retrieve_track_data = rx_request_track.recv().await.unwrap();

        if retrieve_track_data {
            let track = music::get_current_track().await?;
            tx.send(PlaybackStateDelta::Track(track)).await?;
        }

        if let Some(playlist_name) = playlist_name {
            let playlist_duration = music::tell("get {duration} of current playlist")
                .await?
                .parse::<i32>()?;

            tx.send(PlaybackStateDelta::Playlist(Some(Playlist {
                name: playlist_name.clone(),
                duration: playlist_duration,
            })))
            .await?;
        } else {
            tx.send(PlaybackStateDelta::Playlist(None)).await?;
        }
    }

    tx.send(PlaybackStateDelta::Render).await?;

    Ok(())
}

const BAR_CHAR: &str = "━";
#[expect(clippy::cast_possible_truncation, clippy::cast_lossless)]
fn make_bar(n: f64, width: Option<i32>) -> Result<String> {
    let width = width.unwrap_or(30);

    let part_one = (n * (width as f64)).floor() as i32;
    let part_two = width - part_one;

    let mut ret = String::new();
    ret += &BAR_CHAR.repeat(part_one.try_into()?);
    ret += &BAR_CHAR.dimmed().to_string().repeat(part_two.try_into()?);

    Ok(ret)
}

#[expect(clippy::unused_async, clippy::cast_possible_truncation)]
async fn update_display(data: &PlaybackState, options: &NowOptions) -> Result<()> {
    if options.watch {
        execute!(
            stdout(),
            cursor::RestorePosition,
            terminal::Clear(terminal::ClearType::FromCursorDown),
        )?;
    }

    if data.state == PlayerState::Stopped {
        println!("Playback is {}", data.state.red());
    } else if let Some(position) = &data.position
        && let Some(track) = &data.track
    {
        let mut stdout = anstream::stdout().lock();

        writeln!(stdout, "{}", track.name.bold())?;
        writeln!(
            stdout,
            "{} {} {} {}",
            if options.no_nerd_fonts {
                data.state.to_string()
            } else {
                data.state.to_icon()
            },
            format::format_duration(*position as i32, false),
            make_bar(position / track.duration, options.bar_width)?,
            format::format_duration(track.duration as i32, true),
        )?;
        writeln!(
            stdout,
            "{} · {}",
            track.artist.blue(),
            track.album.magenta()
        )?;

        if let Some(playlist) = &data.playlist {
            writeln!(
                stdout,
                "{}",
                format!(
                    "Playlist: {} ({})",
                    playlist.name,
                    format::format_duration_plain(playlist.duration)
                )
                .dimmed()
            )?;
        }

        stdout.flush()?;
    }

    Ok(())
}

async fn receive_delta(
    data: &mut PlaybackState,
    delta: &PlaybackStateDelta,
    options: &NowOptions,
    tx_request_track: &mpsc::Sender<bool>,
) -> Result<()> {
    match delta {
        PlaybackStateDelta::State(state) => {
            data.state = *state;
            data.mystery_counter += 1;
        }

        PlaybackStateDelta::Track(track) => {
            data.track.clone_from(track);
            data.mystery_counter += 1;
        }

        PlaybackStateDelta::Playlist(playlist) => {
            data.playlist.clone_from(playlist);
            data.mystery_counter += 1;
        }

        PlaybackStateDelta::Position(position) => {
            data.position = *position;
            data.mystery_counter += 1;
        }

        PlaybackStateDelta::PositionTick => {
            if data.state == PlayerState::Playing
                && let Some(position) = data.position
            {
                data.position = Some(position + 0.25);
            }
        }

        PlaybackStateDelta::TrackIDRequestMoreInfo(id) => {
            if let Some(track) = &data.track {
                tx_request_track.send(track.id != *id).await.unwrap();
            } else {
                tx_request_track.send(true).await.unwrap();
            }
        }

        PlaybackStateDelta::Render => {
            if options.watch || data.mystery_counter >= 4 {
                update_display(data, options).await?;
            }
        }
    }

    Ok(())
}

pub async fn now(options: NowOptions) -> Result<()> {
    let watch = options.watch;

    if watch {
        execute!(stdout(), cursor::SavePosition)?;
    }

    let (tx, mut rx) = mpsc::channel::<PlaybackStateDelta>(20);
    let (tx_request_track, mut rx_request_track) = mpsc::channel::<bool>(20);

    let (shutdown_tx, shutdown_rx) = watch::channel(());

    let mut tasks = task::JoinSet::<Result<()>>::new();

    tasks.spawn({
        let tx = tx.clone();
        let mut shutdown_rx = shutdown_rx.clone();

        async move {
            let mut intvl = tokio::time::interval(Duration::from_millis(5000));

            loop {
                tokio::select! {
                    _ = intvl.tick() => update_state(&tx, &mut rx_request_track).await?,
                    _ = shutdown_rx.changed() => break,
                }
            }

            Ok(())
        }
    });

    tasks.spawn({
        let mut shutdown_rx = shutdown_rx.clone();
        let tx = tx.clone();

        async move {
            let mut intvl = tokio::time::interval(Duration::from_millis(250));

            loop {
                tokio::select! {
                    _ = intvl.tick() => {
                        tx.send(PlaybackStateDelta::PositionTick).await?;
                        tx.send(PlaybackStateDelta::Render).await?;
                    }
                    _ = shutdown_rx.changed() => break,
                }
            }

            Ok(())
        }
    });

    tasks.spawn({
        let mut shutdown_rx = shutdown_rx.clone();
        let shutdown_tx = shutdown_tx.clone();

        async move {
            let mut local_state = PlaybackState {
                state: PlayerState::Unknown,
                playlist: None,
                position: None,
                track: None,
                mystery_counter: 0,
            };

            loop {
                tokio::select! {
                    delta = rx.recv() => {
                        if let Some(delta) = delta {
                            receive_delta(&mut local_state, &delta, &options, &tx_request_track).await?;

                            if let PlaybackStateDelta::Render = delta
                                && !options.watch
                                && local_state.mystery_counter >= 4 {
                                    let _ = shutdown_tx.send(());
                                }
                        }
                    }
                    _ = shutdown_rx.changed() => break,
                };
            }

            Ok(())
        }
    });

    let _ = tasks
        .join_all()
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>();

    Ok(())
}
