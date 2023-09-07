{
  description = "Minimal Apple Music CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
    ...
  }: let
    version = builtins.substring 0 8 self.lastModifiedDate or "dirty";

    inherit (nixpkgs) lib;

    systems = ["x86_64-darwin" "aarch64-darwin"];

    forAllSystems = fn: lib.genAttrs systems (s: fn nixpkgs.legacyPackages.${s});
  in {
    checks = forAllSystems (pkgs: let
      formatter = self.formatter.${pkgs.system};
    in {
      fmt =
        pkgs.runCommand "check-fmt" {}
        ''
          ${pkgs.lib.getExe formatter} --check ${self}
          touch $out
        '';
    });

    devShells = forAllSystems (pkgs: {
      default = pkgs.mkShell {
        packages = with pkgs; [
          rust-analyzer
          rustc
          cargo
          rustfmt
        ];

        RUST_BACKTRACE = 1;
        RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
      };
    });

    packages = forAllSystems (
      pkgs: let
        scope = lib.makeScope pkgs.newScope;
        fn = final: {p = self.overlays.default final pkgs;};
        inherit (scope fn) p;
      in
        p // {default = p.am;}
    );

    formatter = forAllSystems (p: p.alejandra);

    overlays.default = _: prev: {
      am = prev.callPackage ./default.nix {
        inherit self version;
        inherit (prev.darwin.apple_sdk_11_0.frameworks) CoreFoundation Security;
        inherit (prev.darwin) IOKit;
      };
    };

    homeManagerModules.default = {
      lib,
      config,
      pkgs,
      ...
    }: let
      cfg = config.services.am-discord-rich-presence;
      inherit (lib) mkEnableOption mkIf mkOption mkPackageOption types;
    in {
      options.services.am-discord-rich-presence = {
        enable = mkEnableOption "am-discord-rich-presence";
        package = mkPackageOption pkgs "am" {};

        logFile = mkOption {
          type = types.nullOr types.path;
          default = null;
          description = ''
            Path to where am's Discord presence will store its log file
          '';
          example = ''''${config.xdg.cacheHome}/am-discord-rich-presence.log'';
        };
      };

      config = mkIf cfg.enable {
        assertions = [
          (lib.hm.assertions.assertPlatform
            "launchd.agents.am-discord-rich-presence"
            pkgs
            lib.platforms.darwin)
        ];

        launchd.agents.am-discord-rich-presence = {
          enable = true;

          config = {
            ProgramArguments = ["${lib.getExe cfg.package}" "discord"];
            KeepAlive = true;
            RunAtLoad = true;

            StandardOutPath = cfg.logFile;
            StandardErrorPath = cfg.logFile;
          };
        };
      };
    };
  };
}
