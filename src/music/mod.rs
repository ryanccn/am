use std::process::Stdio;

use tokio::process::Command;

use anyhow::{anyhow, Result};

mod metadata;

pub use metadata::*;

pub async fn is_running() -> Result<bool> {
    Ok(Command::new("pgrep")
        .arg(r"^Music$")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?
        .success())
}

#[derive(Debug, Clone)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub album: String,
    pub artist: String,
    pub duration: f64,
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub name: String,
    pub duration: i32,
}

pub async fn tell_raw(applescript: &[&str]) -> Result<String> {
    let mut osascript_cmd = Command::new("osascript");

    for a in applescript {
        osascript_cmd.arg("-e").arg(a);
    }

    let output = osascript_cmd.output().await?;
    let success = output.status.success();

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !success {
        anyhow::bail!(stderr);
    }

    Ok(stdout)
}

pub async fn tell(applescript: &str) -> Result<String> {
    tell_raw(&["tell application \"Music\"", applescript, "end tell"]).await
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerState {
    Stopped,
    Playing,
    Paused,
    Forwarding,
    Rewinding,
    Unknown,
}

impl PlayerState {
    pub fn to_icon(self) -> String {
        match self {
            Self::Stopped => "",
            Self::Playing => "",
            Self::Paused => "",
            Self::Forwarding => "",
            Self::Rewinding => "",
            Self::Unknown => "?",
        }
        .into()
    }
}

impl std::fmt::Display for PlayerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Stopped => "Stopped",
                Self::Playing => "Playing",
                Self::Paused => "Paused",
                Self::Forwarding => "Fast forwarding",
                Self::Rewinding => "Rewinding",
                Self::Unknown => "Unknown",
            }
        )
    }
}

impl std::str::FromStr for PlayerState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "stopped" => Ok(Self::Stopped),
            "playing" => Ok(Self::Playing),
            "paused" => Ok(Self::Paused),
            "fast forwarding" => Ok(Self::Forwarding),
            "rewinding" => Ok(Self::Rewinding),
            _ => Ok(Self::Unknown),
        }
    }
}

pub async fn get_player_state() -> Result<PlayerState> {
    tell("get player state").await?.parse::<PlayerState>()
}

pub async fn get_current_track() -> Result<Option<Track>> {
    let player_state = get_player_state().await?;

    if player_state == PlayerState::Stopped {
        Ok(None)
    } else {
        let track_data = tell_raw(&[
            "set output to \"\"",
            "tell application \"Music\"",
            "set t_id to database id of current track",
            "set t_name to name of current track",
            "set t_album to album of current track",
            "set t_artist to artist of current track",
            "set t_duration to duration of current track",
            "set output to \"\" & t_id & \"\\n\" & t_name & \"\\n\" & t_album & \"\\n\" & t_artist & \"\\n\" & t_duration",
            "end tell",
            "return output"
        ])
        .await?;

        let mut track_data = track_data.split('\n');

        let id = track_data
            .next()
            .ok_or_else(|| anyhow!("Could not obtain track ID"))?
            .to_owned();
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

        Ok(Some(Track {
            id,
            name,
            album,
            artist,
            duration,
        }))
    }
}
