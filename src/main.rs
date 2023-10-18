use std::io::stdout;

use anyhow::{anyhow, Result};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use owo_colors::OwoColorize;

mod cmd;
mod format;
mod music;
mod rich_presence;

/// Beautiful and feature-packed Apple Music CLI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show now playing
    Now(cmd::NowOptions),

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

    /// Connect to Discord rich presence
    Discord {
        #[command(subcommand)]
        command: Option<DiscordCommands>,
    },

    /// Generate shell completions
    Completions {
        /// Shell
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand, Debug)]
enum DiscordCommands {
    /// Install Discord presence launch agent
    Install,
    /// Uninstall Discord presence launch agent
    Uninstall,
}

#[cfg(not(target_os = "macos"))]
compile_error!("am doesn't work on non-macOS platforms!");

async fn concise_now_playing() -> Result<()> {
    let track_data = music::tell_raw(&[
        "set output to \"\"",
        "tell application \"Music\"",
        "set t_name to name of current track",
        "set t_album to album of current track",
        "set t_artist to artist of current track",
        "set t_duration to duration of current track",
        "set output to \"\" & t_name & \"\\n\" & t_album & \"\\n\" & t_artist & \"\\n\" & t_duration",
        "end tell",
        "return output"
    ])
    .await?;

    let mut track_data = track_data.split('\n');

    let name = track_data
        .next()
        .ok_or_else(|| anyhow!("Could not obtain track name"))?
        .to_owned();
    let album = track_data
        .next()
        .ok_or_else(|| anyhow!("Could not obtain track album"))?
        .to_owned();
    let artist = track_data
        .next()
        .ok_or_else(|| anyhow!("Could not obtain track artist"))?
        .to_owned();
    let duration = track_data
        .next()
        .ok_or_else(|| anyhow!("Could not obtain track duration"))?
        .to_owned()
        .parse::<f64>()?;

    println!(
        "{} {}\n{} · {}",
        name.bold(),
        format::format_duration_plain(&(duration as i32)).dimmed(),
        artist.blue(),
        album.magenta(),
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Play => {
            music::tell("play").await?;
            println!("{} playing music", "Started".green());
            concise_now_playing().await?;
        }
        Commands::Pause => {
            music::tell("pause").await?;
            println!("{} playing music", "Stopped".red());
            concise_now_playing().await?;
        }
        Commands::Toggle => {
            let player_state = music::tell("player state").await?;

            if player_state == "paused" {
                music::tell("play").await?;
                println!("{} playing music", "Started".green());
            } else {
                music::tell("pause").await?;
                println!("{} playing music", "Stopped".red());
            }

            concise_now_playing().await?;
        }

        Commands::Back => {
            music::tell("back track").await?;
            println!("{} to current or previous track", "Back tracked".cyan());
            concise_now_playing().await?;
        }
        Commands::Forward => {
            music::tell("fast forward").await?;
            println!("{} in current track", "Fast forwarded".cyan());
            concise_now_playing().await?;
        }

        Commands::Next => {
            music::tell("next track").await?;
            println!("{} to next track", "Advanced".magenta());
            concise_now_playing().await?;
        }
        Commands::Previous => {
            music::tell("previous track").await?;
            println!("{} to previous track", "Returned".magenta());
            concise_now_playing().await?;
        }

        Commands::Resume => {
            music::tell("resume").await?;
            println!("{} normal playback", "Resumed".magenta());
            concise_now_playing().await?;
        }

        Commands::Now(options) => {
            cmd::now(options).await?;
        }

        Commands::Discord { command } => match command {
            Some(command) => match command {
                DiscordCommands::Install => {
                    cmd::discord::agent::install().await?;
                    println!("{} Discord presence launch agent", "Installed".green());
                }
                DiscordCommands::Uninstall => {
                    cmd::discord::agent::uninstall().await?;
                    println!("{} Discord presence launch agent", "Uninstalled".green());
                }
            },

            None => {
                cmd::discord().await?;
            }
        },

        Commands::Completions { shell } => {
            let cli = &mut Cli::command();
            generate(shell, cli, cli.get_name().to_string(), &mut stdout());
        }
    }

    Ok(())
}
