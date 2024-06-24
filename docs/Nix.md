# Nix

> **Warning**
>
> Nix was not tested and probably will now work
> 
> Try at one's own risk

You can use the flake:

```nix
# flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    anyrun.url = "github:anyrun-org/anyrun";
    anyrun.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, anyrun }: let
  in {
    nixosConfigurations.HOSTNAME = nixpkgs.lib.nixosSystem {
      # ...

      environment.systemPackages = [ anyrun.packages.${system}.anyrun ];

      # ...
    };
  };
}
```

The flake provides multiple packages:

- anyrun (default) - just the anyrun binary
- anyrun-with-all-plugins - anyrun and all builtin plugins
- applications - the applications plugin
- dictionary - the dictionary plugin
- kidex - the kidex plugin
- randr - the randr plugin
- rink - the rink plugin
- shell - the shell plugin
- stdin - the stdin plugin
- symbols - the symbols plugin
- translate - the translate plugin
- websearch - the websearch plugin

#### Home-Manager module

The anyrun flake exposes a Home-Manager module as `homeManagerModules.default`.
You use it in your system like this:

```nix
{
  programs.anyrun = {
    enable = true;
    config = {
      plugins = [
        # An array of all the plugins you want, which either can be paths to the .so files, or their packages
        inputs.anyrun.packages.${pkgs.system}.applications
        ./some_plugin.so
        "${inputs.anyrun.packages.${pkgs.system}.anyrun-with-all-plugins}/lib/kidex"
      ];
      x = { fraction = 0.5; };
      y = { fraction = 0.3; };
      width = { fraction = 0.3; };
      hideIcons = false;
      ignoreExclusiveZones = false;
      layer = "overlay";
      hidePluginInfo = false;
      closeOnClick = false;
      showResultsImmediately = false;
    };
    extraCss = ''
      .some_class {
        background: red;
      }
    '';

    extraConfigFiles."some-plugin.ron".text = ''
      Config(
        // for any other plugin
        // this file will be put in ~/.config/anyrun/some-plugin.ron
        // refer to docs of xdg.configFile for available options
      )
    '';
  };
}
```

You might also want to use the binary cache to avoid building locally.

```nix
nix.settings = {
    builders-use-substitutes = true;
    # extra substituters to add
    extra-substituters = [
        "https://anyrun.cachix.org"
    ];

    extra-trusted-public-keys = [
        "anyrun.cachix.org-1:pqBobmOjI7nKlsUMV25u9QHa9btJK65/C8vnO3p346s="
    ];
};
```
