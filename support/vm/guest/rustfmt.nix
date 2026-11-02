# The nightly rustfmt the tree is formatted with, for this machine: the one the
# dev shell has, from the nixpkgs .nix/flake.lock pins, so the builder VM, smk-1
# and smk format on the Mac format alike and a lock update moves them together.
# Nightly because .rustfmt.toml uses unstable options.
{
  lock ? ../../../.nix/flake.lock,
  system ? builtins.currentSystem,
}:
let
  nodes = (builtins.fromJSON (builtins.readFile lock)).nodes;
  nixpkgs = builtins.fetchTree nodes.${nodes.root.inputs.nixpkgs}.locked;
  pkgs = import nixpkgs {
    inherit system;
    config = { };
    overlays = [ ];
  };
in
pkgs.rustfmt.override { asNightly = true; }
