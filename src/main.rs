use std::io::stdout;

use anyhow::Result;

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
    Now {
        /// Switch to an alternate screen and update now playing until interrupted
        #[arg(short, long)]
        watch: bool,

        /// Disable Nerd Font symbols
        #[arg(long)]
        no_nerd_fonts: bool,

        /// Playback progress bar width
        #[arg(long)]
        bar_width: Option<i32>,
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

    /// Connect to Discord rich presence
    Discord,

    /// Generate shell completions
    Completions {
        /// Shell
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[cfg(not(target_os = "macos"))]
compile_error!("am doesn't work on non-macOS platforms!");

async fn concise_now_playing() -> Result<()> {
    let (name, album, artist, duration) = tokio::try_join!(
        music::tell("get {name} of current track"),
        music::tell("get {album} of current track"),
        music::tell("get {artist} of current track"),
        music::tell("get {duration} of current track")
    )?;
    let duration = duration.parse::<f64>()?;

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

        Commands::Now {
            watch,
            no_nerd_fonts,
            bar_width,
        } => {
            cmd::now(cmd::NowOptions {
                watch,
                no_nerd_fonts,
                bar_width,
            })
            .await?;
        }

        Commands::Discord => {
            cmd::discord().await?;
        }

        Commands::Completions { shell } => {
            let cli = &mut Cli::command();
            generate(shell, cli, cli.get_name().to_string(), &mut stdout());
        }
    }

    Ok(())
}
