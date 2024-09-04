{
  description = "A beautiful and feature-packed Apple Music CLI";

  nixConfig = {
    extra-substituters = [ "https://cache.garnix.io" ];
    extra-trusted-public-keys = [ "cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g=" ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    nix-filter.url = "github:numtide/nix-filter";
  };

  outputs =
    {
      self,
      nixpkgs,
      nix-filter,
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
              nativeBuildInputs ? [ ],
              command,
            }:
            pkgs.stdenv.mkDerivation {
              name = "check-${name}";
              inherit nativeBuildInputs;
              inherit (self.packages.${system}.am) src cargoDeps;

              buildPhase = ''
                ${command}
                touch "$out"
              '';

              doCheck = false;
              dontInstall = true;
              dontFixup = true;
            };
        in
        {
          nixfmt = mkFlakeCheck {
            name = "nixfmt";
            nativeBuildInputs = with pkgs; [ nixfmt-rfc-style ];
            command = "nixfmt --check .";
          };

          rustfmt = mkFlakeCheck {
            name = "rustfmt";

            nativeBuildInputs = with pkgs; [
              cargo
              rustfmt
            ];

            command = "cargo fmt --check";
          };

          clippy = mkFlakeCheck {
            name = "clippy";

            nativeBuildInputs = with pkgs; [
              rustPlatform.cargoSetupHook
              cargo
              rustc
              clippy
              clippy-sarif
              sarif-fmt
            ];

            command = ''
              cargo clippy --all-features --all-targets --tests \
                --offline --message-format=json \
                | clippy-sarif | tee $out | sarif-fmt
            '';
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

              cargo-audit
              cargo-bloat
              cargo-expand

              libiconv
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
        // (lib.attrsets.mapAttrs' (
          name: value: lib.nameValuePair "check-${name}" value
        ) self.checks.${system})
      );

      formatter = forAllSystems (system: nixpkgsFor.${system}.nixfmt-rfc-style);

      overlays.default = _: prev: {
        am = prev.callPackage ./package.nix { inherit nix-filter self; };
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
