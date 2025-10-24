# SPDX-FileCopyrightText: 2025 Ryan Cao <hello@ryanccn.dev>
#
# SPDX-License-Identifier: GPL-3.0-or-later

{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    ferrix.url = "github:ryanccn/ferrix";
  };

  outputs =
    { nixpkgs, ferrix, ... }@inputs:
    ferrix.lib.mkFlake inputs {
      root = ./.;
      systems = nixpkgs.lib.platforms.darwin;

      flake.homeModules = {
        am-discord =
          {
            lib,
            config,
            pkgs,
            ...
          }:
          let
            cfg = config.services.am-discord;
            inherit (lib)
              mkEnableOption
              mkIf
              mkOption
              mkPackageOption
              types
              ;
          in
          {
            options.services.am-discord = {
              enable = mkEnableOption "am-discord";
              package = mkPackageOption pkgs "am" { };

              logFile = mkOption {
                type = types.nullOr types.path;
                default = null;
                description = ''
                  Path to where am's Discord presence will store its log file
                '';
                example = ''''${config.xdg.cacheHome}/am-discord.log'';
              };
            };

            config = mkIf cfg.enable {
              assertions = [
                (lib.hm.assertions.assertPlatform "launchd.agents.am-discord" pkgs lib.platforms.darwin)
              ];

              launchd.agents.am-discord = {
                enable = true;

                config = {
                  ProgramArguments = [
                    "${lib.getExe cfg.package}"
                    "discord"
                  ];
                  KeepAlive = true;
                  RunAtLoad = true;

                  StandardOutPath = cfg.logFile;
                  StandardErrorPath = cfg.logFile;
                };
              };
            };
          };
      };
    };
}
