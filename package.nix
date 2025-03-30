{
  lib,
  rustPlatform,
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
rustPlatform.buildRustPackage (finalAttrs: {
  pname = finalAttrs.passthru.cargoToml.package.name;
  version = finalAttrs.passthru.cargoToml.package.version + "-unstable-${year}-${month}-${day}";

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./src
      ./Cargo.lock
      ./Cargo.toml
    ];
  };

  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [
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
    installShellCompletion --cmd ${finalAttrs.pname} \
      --bash <("$out/bin/${finalAttrs.pname}" completions bash) \
      --zsh <("$out/bin/${finalAttrs.pname}" completions zsh) \
      --fish <("$out/bin/${finalAttrs.pname}" completions fish)
  '';

  passthru.cargoToml = lib.importTOML ./Cargo.toml;

  meta = with lib; {
    description = "A beautiful and feature-packed Apple Music CLI";
    maintainers = with maintainers; [ ryanccn ];
    license = licenses.gpl3Only;
    mainProgram = "am";
  };
})
