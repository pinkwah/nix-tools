{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-24.05-small";
    devenv.url = "github:cachix/devenv";
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-compat.url = "https://flakehub.com/f/edolstra/flake-compat/1.tar.gz";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "x86_64-darwin"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      imports = [ flake-parts.flakeModules.easyOverlay ];

      perSystem =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        {
          overlayAttrs = {
            inherit (config.packages) nix-tools;
          };

          formatter = pkgs.nixfmt-rfc-style;

          packages = rec {
            nix-tools = pkgs.rustPlatform.buildRustPackage {
              name = "nix-tools";
              version = "0.1.0";

              src = ./.;
              cargoSha256 = "sha256-7xjoFfHW7MIcM44XI4PKrOV7+Ok/xNFy9fW0xJ8ZpCc=";
            };

            default = nix-tools;
          };
        };
    };
}
