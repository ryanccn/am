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

### Manual download

Download the [`aarch64`](https://github.com/ryanccn/am/releases/latest/download/am-aarch64-apple-darwin) (Apple Silicon) or the [`x86_64`](https://github.com/ryanccn/am/releases/latest/download/am-x86_64-apple-darwin) (Intel) version of the binary.

Dequarantine them with `xattr -d com.apple.quarantine <path>` and make them executable with `chmod +x <path>`.

## Features

- Beautiful now playing display
- Playback controls (play, pause, toggle, resume, back, forward, next, previous)
- Discord rich presence
- Shell completions

## Home Manager module

This repository's flake also provides a Home Manager module at `homeManagerModules.default`. This module provides a service `am-discord-rich-presence` that you can enable so that `am`'s Discord rich presence runs in the background as long as you are logged in.

```nix
{
  services.am-discord-rich-presence = {
    enable = true;

    # optional
    # logFile = "${config.xdg.cacheHome}/am-discord-rich-presence.log";
  }
}
```
