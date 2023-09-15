# `am`

A beautiful and feature-packed Apple Music CLI!

Written in [Rust](https://www.rust-lang.org/).

## Installation

### Nix (recommended)

This GitHub repository contains a flake. Add `github:ryanccn/am` to your flake inputs:

```nix
{
  inputs = {
    # ...other inputs
    am = {
      url = "github:ryanccn/am";
      inputs.nixpkgs.follows = "nixpkgs";
    }
  }
}
```

Then, use the overlay from `overlays.default` and add `am` to your packages. Alternatively, you can use `packages.default` directly.

### Cargo

You can install (the cutting-edge version of) `am` through `cargo`.

```console
$ cargo install --git https://github.com/ryanccn/am.git
```

### Manual download

Download the [`aarch64`](https://github.com/ryanccn/am/releases/latest/download/am-aarch64-apple-darwin) (Apple Silicon) or the [`x86_64`](https://github.com/ryanccn/am/releases/latest/download/am-x86_64-apple-darwin) (Intel) version of the binary.

Dequarantine them with `xattr -d com.apple.quarantine <path>` and make them executable with `chmod +x <path>`.

## Features

- Beautiful now playing display
- Playback controls (play, pause, toggle, resume, back, forward, next, previous)
- Discord rich presence
- Launch agent installation
- Shell completions

## Screenshots

<details>
  <summary><code>am now</code></summary>

![am now](/.github/images/now.png)

</details>

<details>
  <summary><code>am next</code></summary>

![am next](/.github/images/next.png)

</details>

<details>
  <summary><code>am discord</code></summary>

![am discord](/.github/images/discord.png)

</details>

## Discord presence launch agent

Through a macOS launch agent, the Discord rich presence can be made to run in the background as long as you are logged in.

### Standard installation

You can install the Discord presence as a launch agent by running `am discord install`. Note that this depends on the executable/symlink staying in the same place; if it moves to a different place, run the command again.

The `am` process running in the launch agent will log to `~/Library/Logs/am-discord-rich-presence.log`.

You can uninstall the launch agent with `am discord uninstall`.

### Home Manager

This repository's flake also provides a Home Manager module at `homeManagerModules.default`. This module exposes a service `am-discord-rich-presence` that you can enable.

```nix
{
  services.am-discord-rich-presence = {
    enable = true;

    # optional
    # logFile = "${config.xdg.cacheHome}/am-discord-rich-presence.log";
  }
}
```

## Thanks to...

- [Raycast's Apple Music extension](https://github.com/raycast/extensions/tree/main/extensions/music) for a helpful reference of Apple Music's AppleScript interface usage
- [sardonicism-04/discord-rich-presence](https://github.com/sardonicism-04/discord-rich-presence) for the original Rust crate for connecting to Discord
- [caarlos0/discord-applemusic-rich-presence](https://github.com/caarlos0/discord-applemusic-rich-presence) for inspiring the Discord presence part of this CLI

## License

GPLv3
