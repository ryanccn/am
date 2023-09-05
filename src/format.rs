use anyhow::{anyhow, Result};
use owo_colors::OwoColorize;

const HOUR: f32 = 60. * 60.;
const MINUTE: f32 = 60.;

pub fn format_duration(duration_secs: &f32) -> String {
    let mut duration_secs = *duration_secs;
    let mut str = "".to_owned();
    let mut has_started = false;

    if has_started || duration_secs > HOUR {
        let hours = duration_secs / HOUR;
        let hours_r = hours.floor() as i32;
        str = format!("{}{:.0}{}", str, hours_r, "h".dimmed());
        duration_secs -= (hours_r as f32) * (HOUR);
        has_started = true;
    }

    if has_started || duration_secs > MINUTE {
        let mins = duration_secs / MINUTE;
        let mins_r = mins.floor() as i32;
        str = format!("{}{:.0}{}", str, mins_r, "m".dimmed());
        duration_secs -= (mins_r as f32) * (MINUTE);
        // has_started = true;
    }

    str = format!("{}{:.0}{}", str, duration_secs, "s".dimmed());
    str
}

pub fn format_playlist_duration(duration_secs: &i32) -> String {
    let mut duration_secs = *duration_secs as f32;
    let mut str = "".to_owned();
    let mut has_started = false;

    if has_started || duration_secs > HOUR {
        let hours = duration_secs / HOUR;
        let hours_r = hours.floor() as i32;
        str = format!("{}{:.0}{}", str, hours_r, "h");
        duration_secs -= (hours_r as f32) * (HOUR);
        has_started = true;
    }

    if has_started || duration_secs > MINUTE {
        let mins = duration_secs / MINUTE;
        let mins_r = mins.floor() as i32;
        str = format!("{}{:.0}{}", str, mins_r, "m");
        duration_secs -= (mins_r as f32) * (MINUTE);
        // has_started = true;
    }

    str = format!("{}{:.0}{}", str, duration_secs, "s");
    str
}

pub fn format_player_state(raw: &str) -> Result<String> {
    match raw {
        "stopped" => Ok("Stopped".into()),
        "playing" => Ok("Playing".into()),
        "paused" => Ok("Paused".into()),
        "fast forwarding" => Ok("Fast forwarding".into()),
        "rewinding" => Ok("Rewinding".into()),
        &_ => Err(anyhow!("Unexpected player state {}", raw)),
    }
}
