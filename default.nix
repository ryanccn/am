{
  lib,
  stdenv,
  rustPlatform,
  darwin,
  nix-filter,
  pkg-config,
  installShellFiles,
  self,
  enableLTO ? true,
  enableOptimizeSize ? false,
}:
let
  year = builtins.substring 0 4 self.lastModifiedDate;
  month = builtins.substring 4 2 self.lastModifiedDate;
  day = builtins.substring 6 2 self.lastModifiedDate;
in
rustPlatform.buildRustPackage rec {
  pname = passthru.cargoToml.package.name;
  version = passthru.cargoToml.package.version + "-unstable-${year}-${month}-${day}";

  strictDeps = true;

  src = nix-filter.lib.filter {
    root = self;
    include = [
      "src"
      "Cargo.lock"
      "Cargo.toml"
    ];
  };

  cargoLock.lockFile = ./Cargo.lock;

  buildInputs = lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.CoreFoundation
    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
    darwin.apple_sdk.frameworks.IOKit
    darwin.libiconv
  ];

  nativeBuildInputs = lib.optionals stdenv.isDarwin [
    pkg-config
    installShellFiles
  ];

  env =
    lib.optionalAttrs enableLTO {
      CARGO_PROFILE_RELEASE_LTO = "fat";
      CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "1";
    }
    // lib.optionalAttrs enableOptimizeSize {
      CARGO_PROFILE_RELEASE_OPT_LEVEL = "z";
      CARGO_PROFILE_RELEASE_PANIC = "abort";
      CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "1";
      CARGO_PROFILE_RELEASE_STRIP = "symbols";
    };

  postInstall = ''
    installShellCompletion --cmd ${pname} \
      --bash <("$out/bin/${pname}" completions bash) \
      --zsh <("$out/bin/${pname}" completions zsh) \
      --fish <("$out/bin/${pname}" completions fish)
  '';

  passthru.cargoToml = lib.importTOML ./Cargo.toml;

  meta = with lib; {
    description = "A beautiful and feature-packed Apple Music CLI";
    maintainers = with maintainers; [ ryanccn ];
    license = licenses.gpl3Only;
    mainProgram = "am";
  };
}
