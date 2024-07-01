{
  description = "A wayland native, highly customizable runner.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default-linux";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      flake-parts,
      systems,
      ...
    }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ flake-parts.flakeModules.easyOverlay ];
      systems = import systems;

      perSystem =
        {
          self',
          config,
          pkgs,
          ...
        }:
        let
          inherit (pkgs) callPackage;
        in
        rec {
          formatter = pkgs.nixfmt-rfc-style;

          devShells.default = pkgs.mkShell {
            inputsFrom = builtins.attrValues self'.packages;

            packages = with pkgs; [
              rustfmt # rust formatter
              statix # lints and suggestions
              deadnix # clean up unused nix code
              rustc # rust compiler
              cargo # rust package manager
              clippy # opinionated rust formatter
            ];
          };

          checks = packages;
          packages =
            let
              cargoLock.lockFile = ./Cargo.lock;
              plugins = builtins.attrNames (builtins.readDir ./plugins);
            in
            rec {
              default = anyrun;
              anyrun = callPackage ./nix/default.nix { inherit cargoLock; };
              anyrun-with-all-plugins = callPackage ./nix/default.nix { inherit cargoLock plugins; };
            }
            // builtins.listToAttrs (
              builtins.map (name: {
                inherit name;
                value = callPackage ./nix/plugin.nix { inherit name cargoLock; };
              }) plugins
            );

          # Set up an overlay from packages exposed by this flake
          overlayAttrs = config.packages;
        };

      flake.homeManagerModules = rec {
        default = anyrun;
        anyrun = import ./nix/hm-module.nix self;
      };
    };
}
