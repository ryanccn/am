use std::process::Command;

use anyhow::Result;

pub fn tell_music(applescript: &str) -> Result<String> {
    let mut osascript_cmd = Command::new("osascript");
    osascript_cmd.arg("-e").arg("tell application \"Music\"");
    osascript_cmd.arg("-e").arg(applescript);
    osascript_cmd.arg("-e").arg("end tell");

    let output = osascript_cmd.output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
