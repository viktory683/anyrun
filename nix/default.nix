{
  lib,
  makeWrapper,
  lockFile,
  # Dependencies for Anyrun
  glib,
  gtk4,
  gtk4-layer-shell,
  rustPlatform,
  atk,
  pkg-config,
  librsvg,
  rustfmt,
  cargo,
  rustc,
  # Additional configuration arguments for the
  # derivation. By default, we should not build
  # any of the plugins.
  dontBuildPlugins ? true,
  ...
}: let
  inherit (builtins) fromTOML readFile;
  cargoToml = fromTOML (readFile ../anyrun/Cargo.toml);
  pname = cargoToml.package.name;
  version = cargoToml.package.version;
in
  rustPlatform.buildRustPackage {
    inherit pname version;
    src = ../.;

    buildInputs = [
      glib
      atk
      librsvg
      gtk4
      gtk4-layer-shell
    ];

    cargoLock = {
      inherit lockFile;
    };

    checkInputs = [
      cargo
      rustc
    ];

    nativeBuildInputs = [
      pkg-config
      makeWrapper
      rustfmt
      rustc
      cargo
    ];

    cargoBuildFlags =
      if dontBuildPlugins
      then ["-p ${pname}"]
      else [];

    doCheck = true;
    CARGO_BUILD_INCREMENTAL = "false";
    RUST_BACKTRACE = "full";
    copyLibs = true;
    buildAndTestSubdir =
      if dontBuildPlugins
      then pname
      else null;

    postInstall = ''
      wrapProgram $out/bin/anyrun \
        --set GDK_PIXBUF_MODULE_FILE "$(echo ${librsvg.out}/lib/gdk-pixbuf-2.0/*/loaders.cache)" \
        --prefix ANYRUN_PLUGINS : $out/lib
    '';

    meta = {
      description = "A wayland native, highly customizable runner.";
      homepage = "https://github.com/bzglve/anyrun";
      license = with lib.licenses; [gpl3];
    };
  }
