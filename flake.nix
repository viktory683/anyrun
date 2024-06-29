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

  outputs = {
    self,
    flake-parts,
    nixpkgs,
    systems,
    ...
  } @ inputs:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [flake-parts.flakeModules.easyOverlay];
      systems = import systems;

      perSystem = {
        self',
        config,
        pkgs,
        ...
      }: let
        inherit (pkgs) callPackage;
      in rec {
        # provide the formatter for nix fmt
        formatter = pkgs.alejandra;

        devShells.default = pkgs.mkShell {
          inputsFrom = builtins.attrValues self'.packages;

          packages = with pkgs; [
            alejandra # nix formatter
            rustfmt # rust formatter
            statix # lints and suggestions
            deadnix # clean up unused nix code
            rustc # rust compiler
            gcc
            cargo # rust package manager
            clippy # opinionated rust formatter
          ];
        };
  
        checks = packages;
        packages = let
          lockFile = ./Cargo.lock;
          mkPlugin = name:
            callPackage ./nix/plugins/default.nix {
              inherit lockFile name;
            };
        in rec {
          default = anyrun;
          anyrun = callPackage ./nix/default.nix {inherit lockFile;};

          anyrun-with-all-plugins = pkgs.callPackage ./nix/default.nix {
            inherit lockFile;
            dontBuildPlugins = false;
          };

          applications = mkPlugin "applications";
          dictionary = mkPlugin "dictionary";
          kidex = mkPlugin "kidex";
          randr = mkPlugin "randr";
          rink = mkPlugin "rink";
          shell = mkPlugin "shell";
          stdin = mkPlugin "stdin";
          symbols = mkPlugin "symbols";
          translate = mkPlugin "translate";
          websearch = mkPlugin "websearch";
        };

        # Set up an overlay from packages exposed by this flake
        overlayAttrs = config.packages;
      };

      flake = {
        homeManagerModules = rec {
          default = anyrun;
          anyrun = import ./nix/hm-module.nix self;
        };
      };
    };
}
