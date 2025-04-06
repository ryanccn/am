use crate::{
    format,
    music::{self, PlayerState, Playlist, Track},
};

use anstream::{eprintln, println};
use clap::Parser;
use crossterm::{cursor, execute, terminal};
use eyre::{Result, eyre};
use owo_colors::OwoColorize as _;

use std::{
    io::{Write as _, stdout},
    time::Duration,
};
use tokio::{signal::ctrl_c, sync::mpsc, task};

#[derive(Parser, Debug)]
pub struct NowOptions {
    /// Switch to an alternate screen and update now playing until interrupted
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

        let player_position = player_position.parse::<f64>().ok();

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
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All)
        )?;
    }

    if data.state == PlayerState::Stopped {
        println!("Playback is {}", data.state.red());
    } else {
        let position = data
            .position
            .ok_or_else(|| eyre!("Could not obtain position from shared playback state"))?;
        let track = data
            .track
            .clone()
            .ok_or_else(|| eyre!("Could not obtain track data from shared playback state"))?;

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
            format::format_duration(position as i32, false),
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
        }

        PlaybackStateDelta::Track(track) => {
            data.track.clone_from(track);
        }
        PlaybackStateDelta::Playlist(playlist) => {
            data.playlist.clone_from(playlist);
        }

        PlaybackStateDelta::Position(position) => {
            data.position = *position;
        }

        PlaybackStateDelta::PositionTick => {
            if data.state == PlayerState::Playing {
                if let Some(position) = data.position {
                    data.position = Some(position + 0.25);
                }
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
            if let Err(err) = update_display(data, options).await {
                eprintln!("{err}");
            }
        }
    }

    Ok(())
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub async fn now(options: NowOptions) -> Result<()> {
    let watch = options.watch;

    if watch {
        execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;
    }

    let (tx, mut rx) = mpsc::channel::<PlaybackStateDelta>(20);
    let (tx_request_track, mut rx_request_track) = mpsc::channel::<bool>(20);

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());

    let mut tasks = task::JoinSet::<()>::new();

    tasks.spawn({
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

    tasks.spawn({
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

    tasks.spawn({
        let mut shutdown_rx = shutdown_rx.clone();

        async move {
            let mut local_state = PlaybackState {
                state: PlayerState::Unknown,
                playlist: None,
                position: None,
                track: None,
            };

            loop {
                tokio::select! {
                    delta = rx.recv() => {
                        if let Some(delta) = delta {
                            receive_delta(&mut local_state, &delta, &options, &tx_request_track).await.unwrap();

                            if let PlaybackStateDelta::Render = delta  {
                                if !options.watch {
                                    break;
                                }
                            }
                        }
                    }

                    _ = shutdown_rx.changed() => break,
                };
            }
        }
    });

    tasks.spawn(async move {
        if ctrl_c().await.is_ok() {
            let _ = shutdown_tx.send(());
        }
    });

    tasks.join_all().await;

    if watch {
        execute!(stdout(), terminal::LeaveAlternateScreen, cursor::Show)?;
    }

    Ok(())
}
