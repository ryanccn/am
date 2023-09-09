use crate::{format, music};

use anyhow::Result;
use crossterm::{cursor, execute, terminal};
use owo_colors::OwoColorize;

use std::{io::stdout, time::Duration};
use tokio::signal::ctrl_c;

struct Playlist {
    name: String,
    duration: i32,
}

async fn update_now(watch: bool) -> Result<()> {
    let player_state = music::tell("player state").await?;

    if player_state == "stopped" {
        println!("Playback is {}", "stopped".red());
    } else {
        let (
            track_name,
            track_album,
            track_artist,
            track_duration_str,
            player_position_str,
            playlist_name,
        ) = tokio::try_join!(
            music::tell("get {name} of current track"),
            music::tell("get {album} of current track"),
            music::tell("get {artist} of current track"),
            music::tell("get {duration} of current track"),
            music::tell("player position"),
            music::tell("get {name} of current playlist")
        )?;

        let track_duration = track_duration_str.parse::<f32>()?;
        let player_position = player_position_str.parse::<f32>()?;

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

        if watch {
            execute!(
                stdout(),
                cursor::MoveTo(0, 0),
                terminal::Clear(terminal::ClearType::All)
            )?;
        }

        println!("{}", track_name.bold());
        println!(
            "{} {}/{}",
            format::format_player_state(&player_state)?,
            format::format_duration(&player_position, false),
            format::format_duration(&track_duration, true),
        );
        println!("{} · {}", track_artist.blue(), track_album.magenta());

        if let Some(playlist) = playlist {
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
    }

    Ok(())
}

pub async fn now(watch: bool) -> Result<()> {
    if watch {
        execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;
    }

    let mut intvl = tokio::time::interval(Duration::from_millis(500));

    if watch {
        loop {
            tokio::select! {
                _ = intvl.tick() => {
                    update_now(watch).await.or_else(|r| {
                        execute!(stdout(), terminal::LeaveAlternateScreen, cursor::Show)?;
                        Err(r)
                    })?;
                }
                _ = ctrl_c() => {
                    break;
                }
            }
        }
    } else {
        update_now(watch).await?;
    }

    if watch {
        execute!(stdout(), terminal::LeaveAlternateScreen, cursor::Show)?;
    }

    Ok(())
}
