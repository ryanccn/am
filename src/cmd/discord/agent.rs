// SPDX-FileCopyrightText: 2025 Ryan Cao <hello@ryanccn.dev>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::{Path, PathBuf};
use tokio::{fs, process::Command};

use anstream::println;
use eyre::Result;
use owo_colors::OwoColorize as _;

const AGENT_ID: &str = "dev.ryanccn.am.discord";

fn get_agent_path() -> Result<PathBuf> {
    Ok(Path::new(&std::env::var("HOME")?)
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{AGENT_ID}.plist")))
}

fn get_plist() -> Result<String> {
    let executable_path = std::env::current_exe()?;
    let executable = executable_path.to_string_lossy();

    let log_file_path = PathBuf::from(std::env::var("HOME")?)
        .join("Library")
        .join("Logs")
        .join("am-discord.log");
    let log_file = log_file_path.to_string_lossy();

    Ok(format!(r#"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>KeepAlive</key>
	<true/>
	<key>Label</key>
	<string>{AGENT_ID}</string>
	<key>ProgramArguments</key>
	<array>
		<string>{executable}</string>
		<string>discord</string>
	</array>
	<key>RunAtLoad</key>
	<true/>
    <key>StandardOutPath</key>
	<string>{log_file}</string>
	<key>StandardErrorPath</key>
	<string>{log_file}</string>
</dict>
</plist>
    "#).trim().to_owned() + "\n")
}

pub async fn install() -> Result<()> {
    let path = get_agent_path()?;

    if path.exists() {
        uninstall().await?;
    }

    fs::write(&path, get_plist()?).await?;

    Command::new("launchctl")
        .args(["load", "-w", &path.to_string_lossy()])
        .status()
        .await?;

    Ok(())
}

pub async fn uninstall() -> Result<()> {
    let path = get_agent_path()?;

    if !path.exists() {
        println!("{}", "Launch agent is not installed".yellow());
        return Ok(());
    }

    Command::new("launchctl")
        .args(["unload", &path.to_string_lossy()])
        .status()
        .await?;

    fs::remove_file(&path).await?;

    Ok(())
}
