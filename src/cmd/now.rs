use crate::{format, music};

use anyhow::{anyhow, Result};
use crossterm::{cursor, execute, terminal};
use owo_colors::OwoColorize;

use std::{io::stdout, time::Duration};
use tokio::{signal::ctrl_c, sync::mpsc};

#[derive(Debug, Clone)]
pub struct NowOptions {
    pub watch: bool,
    pub no_nerd_fonts: bool,
    pub bar_width: Option<i32>,
}

#[derive(Debug, Clone)]
struct PlaybackState {
    state: String,
    position: Option<f32>,
    track: Option<Track>,
    playlist: Option<Playlist>,
}

#[derive(Debug)]
enum PlaybackStateDelta {
    State(String),
    Position(Option<f32>),
    PositionTick,
    TrackIDRequestMoreInfo(String),
    Track(Option<Track>),
    Playlist(Option<Playlist>),
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

async fn update_state(
    tx: &mpsc::Sender<PlaybackStateDelta>,
    rx_request_track: &mut mpsc::Receiver<bool>,
) -> Result<()> {
    let player_state = music::tell("player state").await?;
    tx.send(PlaybackStateDelta::State(player_state.clone()))
        .await?;

    if player_state != "stopped" {
        let (track_id, player_position, playlist_name) = tokio::try_join!(
            music::tell("get {database id} of current track"),
            music::tell("player position"),
            music::tell("get {name} of current playlist"),
        )?;

        let player_position = player_position.parse::<f32>().ok();

        tx.send(PlaybackStateDelta::Position(player_position))
            .await?;

        tx.send(PlaybackStateDelta::TrackIDRequestMoreInfo(track_id.clone()))
            .await?;
        let retrieve_track_data = rx_request_track.try_recv()?;

        if retrieve_track_data {
            let (track_name, track_album, track_artist, track_duration_str) = tokio::try_join!(
                music::tell("get {name} of current track"),
                music::tell("get {album} of current track"),
                music::tell("get {artist} of current track"),
                music::tell("get {duration} of current track")
            )?;

            let track_duration = track_duration_str.parse::<f32>()?;

            tx.send(PlaybackStateDelta::Track(Some(Track {
                id: track_id,
                name: track_name,
                album: track_album,
                artist: track_artist,
                duration: track_duration,
            })))
            .await?;
        }

        if !playlist_name.is_empty() {
            let playlist_duration = music::tell("get {duration} of current playlist")
                .await?
                .parse::<i32>()?;

            tx.send(PlaybackStateDelta::Playlist(Some(Playlist {
                name: playlist_name.to_string(),
                duration: playlist_duration,
            })))
            .await?;
        } else {
            tx.send(PlaybackStateDelta::Playlist(None)).await?;
        };
    }

    Ok(())
}

const BAR_CHAR: &str = "━";
fn make_bar(n: f32, width: Option<i32>) -> Result<String> {
    let width = width.unwrap_or(20);

    let part_one = (n * (width as f32)).floor() as i32;
    let part_two = width - part_one;

    let mut ret = "".to_owned();
    ret += &BAR_CHAR.repeat(part_one.try_into()?);
    ret += &BAR_CHAR.dimmed().to_string().repeat(part_two.try_into()?);

    Ok(ret)
}

async fn update_display(data: &PlaybackState, options: &NowOptions) -> Result<()> {
    if options.watch {
        execute!(
            stdout(),
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All)
        )?;
    }

    if data.state == "stopped" {
        println!("Playback is {}", data.state.red());
    } else {
        let position = data
            .position
            .clone()
            .ok_or_else(|| anyhow!("Could not obtain position from shared playback state"))?;
        let track = data
            .track
            .clone()
            .ok_or_else(|| anyhow!("Could not obtain track data from shared playback state"))?;

        println!("{}", track.name.bold());
        println!(
            "{} {} {} {}",
            format::format_player_state(&data.state, !options.no_nerd_fonts)?,
            format::format_duration(&(position as i32), false),
            make_bar(position / track.duration, options.bar_width)?,
            format::format_duration(&(track.duration as i32), true),
        );
        println!("{} · {}", track.artist.blue(), track.album.magenta());

        if let Some(playlist) = &data.playlist {
            println!(
                "{}",
                format!(
                    "Playlist: {} ({})",
                    playlist.name,
                    format::format_duration_plain(&playlist.duration)
                )
                .dimmed()
            );
        } else {
            println!("{}", "No playlist".dimmed());
        };
    };

    Ok(())
}

pub async fn now(options: NowOptions) -> Result<()> {
    if options.watch {
        execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;
    };

    let (tx, mut rx) = mpsc::channel::<PlaybackStateDelta>(5);
    let (tx_request_track, mut rx_request_track) = mpsc::channel::<bool>(1);

    if options.watch {
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());

        let state_task = tokio::spawn({
            let tx = tx.clone();
            let mut shutdown_rx = shutdown_rx.clone();

            async move {
                let mut intvl = tokio::time::interval(Duration::from_secs(1));

                loop {
                    tokio::select! {
                        _ = intvl.tick() => if let Err(err) = update_state(&tx, &mut rx_request_track).await {
                            eprintln!("{err}");
                        },
                        _ = shutdown_rx.changed() => break,
                    }
                }
            }
        });

        let playback_tick_period_ms = 250.0;

        let playback_tick_task = tokio::spawn({
            let mut shutdown_rx = shutdown_rx.clone();
            let tx = tx.clone();

            async move {
                let mut intvl =
                    tokio::time::interval(Duration::from_millis(playback_tick_period_ms as u64));

                loop {
                    tokio::select! {
                        _ = intvl.tick() => {
                            tx.send(PlaybackStateDelta::PositionTick).await.unwrap();
                        }
                        _ = shutdown_rx.changed() => break,
                    }
                }
            }
        });

        let display_task = tokio::spawn({
            let mut shutdown_rx = shutdown_rx.clone();
            let options = options.clone();

            async move {
                let mut local_state = PlaybackState {
                    state: "stopped".into(),
                    playlist: None,
                    position: None,
                    track: None,
                };

                loop {
                    tokio::select! {
                        data = rx.recv() => {
                            if let Some(data) = data {
                                match data {
                                    PlaybackStateDelta::State(state) => {
                                        local_state.state = state;
                                    }

                                    PlaybackStateDelta::Track(track) => {
                                        local_state.track = track;
                                    }
                                    PlaybackStateDelta::Playlist(playlist) => {
                                        local_state.playlist = playlist;
                                    }

                                    PlaybackStateDelta::Position(position) => {
                                        local_state.position = position;
                                    }
                                    PlaybackStateDelta::PositionTick => {
                                        if let Some(position) = local_state.position {
                                            local_state.position = Some(position + 0.25);
                                        }
                                    }

                                    PlaybackStateDelta::TrackIDRequestMoreInfo(id) => {
                                        if let Some(track) = &local_state.track {
                                            tx_request_track.send(track.id != id).await.unwrap();
                                        } else {
                                            tx_request_track.send(true).await.unwrap();
                                        };
                                    }
                                };
                            };

                            if let Err(err) = update_display(&local_state, &options).await {
                                eprintln!("{}", err);
                            };
                        }

                        _ = shutdown_rx.changed() => break,
                    };
                }
            }
        });

        let ctrlc_task = tokio::spawn(async move {
            if ctrl_c().await.is_ok() {
                let _ = shutdown_tx.send(());
            }
        });

        tokio::try_join!(state_task, playback_tick_task, display_task, ctrlc_task)?;
    };

    if options.watch {
        execute!(stdout(), terminal::LeaveAlternateScreen, cursor::Show)?;
    }

    Ok(())
}
