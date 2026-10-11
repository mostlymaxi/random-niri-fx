{
  lib,
  rustPlatform,
  makeWrapper,
  coreutils,
}:

let
  cargo = lib.importTOML ./Cargo.toml;
in
rustPlatform.buildRustPackage {
  pname = cargo.package.name;
  inherit (cargo.package) version;

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };

  cargoLock.lockFile = ./Cargo.lock;

  # Local time comes from date(1), looked up on PATH at runtime, which is
  # minimal under a systemd user service.
  nativeBuildInputs = [ makeWrapper ];
  postInstall = ''
    wrapProgram $out/bin/random-fx --prefix PATH : ${lib.makeBinPath [ coreutils ]}
  '';

  meta = {
    inherit (cargo.package) description;
    mainProgram = "random-fx";
    platforms = lib.platforms.linux;
  };
}
