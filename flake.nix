{
  description = "A beautiful and feature-packed Apple Music CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    {
      self,
      nixpkgs,
    }:
    let
      inherit (nixpkgs) lib;
      systems = [
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forAllSystems = lib.genAttrs systems;
      nixpkgsFor = forAllSystems (system: nixpkgs.legacyPackages.${system});
    in
    {
      checks = forAllSystems (
        system:
        let
          pkgs = nixpkgsFor.${system};

          mkFlakeCheck =
            {
              name,
              command,
              ...
            }@args:
            pkgs.stdenv.mkDerivation (
              {
                name = "check-${name}";
                inherit (self.packages.${system}.am) src;

                buildPhase = ''
                  ${command}
                  touch "$out"
                '';

                doCheck = false;
                dontInstall = true;
                dontFixup = true;
              }
              // (removeAttrs args [
                "name"
                "command"
              ])
            );
        in
        {
          nixfmt = mkFlakeCheck {
            name = "nixfmt";
            command = "find . -name '*.nix' -exec nixfmt --check {} +";

            src = self;
            nativeBuildInputs = with pkgs; [ nixfmt-rfc-style ];
          };

          rustfmt = mkFlakeCheck {
            name = "rustfmt";
            command = "cargo fmt --check";

            nativeBuildInputs = with pkgs; [
              cargo
              rustfmt
            ];
          };

          clippy = mkFlakeCheck {
            name = "clippy";
            command = ''
              cargo clippy --all-features --all-targets --tests \
                --offline --message-format=json \
                | clippy-sarif | tee $out | sarif-fmt
            '';

            nativeBuildInputs = with pkgs; [
              rustPlatform.cargoSetupHook
              cargo
              rustc
              clippy
              clippy-sarif
              sarif-fmt
            ];

            inherit (self.packages.${system}.am) cargoDeps;
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgsFor.${system};
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rustfmt
              clippy
              rust-analyzer
            ];

            inputsFrom = [ self.packages.${system}.am ];

            env = {
              RUST_BACKTRACE = 1;
              RUST_SRC_PATH = toString pkgs.rustPlatform.rustLibSrc;
            };
          };
        }
      );

      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgsFor.${system};
          packages = self.overlays.default null pkgs;
        in
        {
          inherit (packages) am;
          default = packages.am;
        }
      );

      formatter = forAllSystems (system: nixpkgsFor.${system}.nixfmt-rfc-style);

      overlays.default = _: prev: {
        am = prev.callPackage ./package.nix { inherit self; };
      };

      homeManagerModules.default =
        {
          lib,
          config,
          pkgs,
          ...
        }:
        let
          cfg = config.services.am-discord-rich-presence;
          inherit (lib)
            mkEnableOption
            mkIf
            mkOption
            mkPackageOption
            types
            ;
        in
        {
          options.services.am-discord-rich-presence = {
            enable = mkEnableOption "am-discord-rich-presence";
            package = mkPackageOption pkgs "am" { };

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
              (lib.hm.assertions.assertPlatform "launchd.agents.am-discord-rich-presence" pkgs
                lib.platforms.darwin
              )
            ];

            launchd.agents.am-discord-rich-presence = {
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
}
