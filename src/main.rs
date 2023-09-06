use std::{io::stdout, thread::sleep, time::Duration};

use anyhow::{anyhow, Result};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};

use crossterm::{cursor, execute, terminal};
use owo_colors::OwoColorize;

mod format;
mod ipc;

/// Minimal Apple Music CLI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show now playing
    Now {
        /// Switch to an alternate screen and update now playing until interrupted
        #[arg(short, long, default_value_t = false)]
        watch: bool,
    },

    /// Play the current track
    Play,
    /// Pause playback
    Pause,

    /// Toggle playing status
    #[command(visible_aliases = ["p"])]
    Toggle,

    /// Disable fast forward/rewind and resume playback
    Resume,

    /// Reposition to beginning of current track or go to previous track if already at start of current track
    Back,

    /// Skip forward in the current track
    Forward,

    /// Advance to the next track in the current playlist
    Next,

    /// Return to the previous track in the current playlist
    #[command(visible_aliases = ["prev"])]
    Previous,

    /// Generate shell completions
    Completions {
        /// Shell
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn check_os() -> Result<()> {
    if std::env::consts::OS != "macos" {
        return Err(anyhow!(
            "`am` is not supported on {}!",
            std::env::consts::OS
        ));
    }

    Ok(())
}

struct Playlist {
    name: String,
    duration: i32,
}

fn main() -> Result<()> {
    check_os()?;

    let args = Cli::parse();

    match args.command {
        Commands::Play => {
            ipc::tell_music("play")?;
            println!("{} playing music", "Started".green());
        }
        Commands::Pause => {
            ipc::tell_music("pause")?;
            println!("{} playing music", "Stopped".red());
        }
        Commands::Toggle => {
            let player_state = ipc::tell_music("player state")?;

            if player_state == "paused" {
                ipc::tell_music("play")?;
                println!("{} playing music", "Started".green());
            } else {
                ipc::tell_music("pause")?;
                println!("{} playing music", "Stopped".red());
            }
        }

        Commands::Back => {
            ipc::tell_music("back track")?;
            println!("{} to current or previous track", "Back tracked".cyan());
        }

        Commands::Forward => {
            ipc::tell_music("fast forward")?;
            println!("{} in current track", "Fast forwarded".cyan());
        }
        Commands::Next => {
            ipc::tell_music("next track")?;
            println!("{} to next track", "Advanced".magenta());
        }

        Commands::Previous => {
            ipc::tell_music("previous track")?;
            println!("{} to previous track", "Returned".magenta());
        }
        Commands::Resume => {
            ipc::tell_music("resume")?;
            println!("{} normal playback", "Resumed".magenta());
        }

        Commands::Now { watch } => loop {
            let player_state = ipc::tell_music("player state")?;

            if watch {
                execute!(stdout(), terminal::EnterAlternateScreen)?;
            }

            if player_state == "stopped" {
                println!("Playback is {}", "stopped".red());
            } else {
                let track_name = ipc::tell_music("get {name} of current track")?;
                let track_album = ipc::tell_music("get {album} of current track")?;
                let track_artist = ipc::tell_music("get {artist} of current track")?;

                let player_position_str = ipc::tell_music("player position")?;
                let player_position = player_position_str.parse::<f32>()?;

                let playlist_name = ipc::tell_music("get {name} of current playlist").ok();
                let mut playlist: Option<Playlist> = None;

                if let Some(playlist_name) = &playlist_name {
                    if !playlist_name.is_empty() {
                        let playlist_duration =
                            ipc::tell_music("get {duration} of current playlist")?
                                .parse::<i32>()?;

                        playlist = Some(Playlist {
                            name: playlist_name.to_string(),
                            duration: playlist_duration,
                        });
                    }
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
                    "{} {}",
                    format::format_player_state(&player_state)?,
                    format::format_duration(&player_position)
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

            if watch {
                sleep(Duration::from_millis(500));
            } else {
                break;
            }
        },
        Commands::Completions { shell } => {
            let cli = &mut Cli::command();
            generate(shell, cli, cli.get_name().to_string(), &mut stdout());
        }
    }

    Ok(())
}
