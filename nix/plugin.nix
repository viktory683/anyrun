{
  lib,
  glib,
  rustPlatform,
  atk,
  librsvg,
  name,
  cargoLock,
  ...
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../plugins/${name}/Cargo.toml);
  pname = cargoToml.package.name;
  version = cargoToml.package.version;
in
rustPlatform.buildRustPackage {
  inherit pname version cargoLock;
  src = ../.;

  copyLibs = true;
  CARGO_INCREMENTAL = 0;
  cargoBuildFlags = [ "-p ${name}" ];
  buildAndTestSubdir = "plugins/${name}";

  buildInputs = [
    glib
    atk
    librsvg
  ];

  meta = {
    description = "The ${name} plugin for Anyrun";
    homepage = "https://github.com/bzglve/anyrun";
    license = [ lib.licenses.gpl3 ];
  };
}
