{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

{
  languages.rust.enable = true;

  pre-commit.hooks = {
    nixfmt-rfc-style.enable = true;
    rustfmt.enable = true;
  };
}
