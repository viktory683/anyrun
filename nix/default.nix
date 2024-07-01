{
  lib,
  makeWrapper,
  cargoLock,
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
  wrapGAppsHook4,
  plugins ? [ ],
  ...
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../anyrun/Cargo.toml);
  pname = cargoToml.package.name;
  version = cargoToml.package.version;
in
rustPlatform.buildRustPackage {
  inherit pname version cargoLock;
  src = ../.;

  copyLibs = true;
  CARGO_INCREMENTAL = 0;
  cargoBuildFlags = [ "-p ${pname}" ] ++ builtins.map (plugin: "-p ${plugin}") plugins;
  buildAndTestSubdir = lib.optionalString ((builtins.length plugins) == 0) pname;

  buildInputs = [
    glib
    atk
    librsvg
    gtk4
    gtk4-layer-shell
  ];

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
    wrapGAppsHook4
  ];

  postInstall = ''
    glib_dir=$out/share/glib-2.0/schemas
    mkdir -p $glib_dir
    cp settings/1/* $glib_dir
    glib-compile-schemas $glib_dir
  '';

  preFixup = ''
    gappsWrapperArgs+=(
      --set GDK_PIXBUF_MODULE_FILE "$(echo ${librsvg.out}/lib/gdk-pixbuf-2.0/*/loaders.cache)" 
      --prefix ANYRUN_PLUGINS : "$out/lib" 
    )
  '';

  meta = {
    description = "A wayland native, highly customizable runner.";
    homepage = "https://github.com/bzglve/anyrun";
    license = [ lib.licenses.gpl3 ];
  };
}
