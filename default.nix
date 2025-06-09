# This file provides a default.nix for backwards compatibility with non-flake usage
{ pkgs ? import <nixpkgs> { } }:

let
  flake = import ./flake.nix;
  system = pkgs.system or builtins.currentSystem;
  outputs = flake.outputs {
    self = flake;
    nixpkgs = pkgs;
    flake-utils = import (pkgs.fetchFromGitHub {
      owner = "numtide";
      repo = "flake-utils";
      rev = "b1d9ab70662946ef0850d488da1c9019f3a9752a";
      sha256 = "sha256-Pjj1zp1RqLhKj+SBpYPkIJGJjt30QbmTHa1lEKQQks4=";
    });
    rust-overlay = import (pkgs.fetchFromGitHub {
      owner = "oxalica";
      repo = "rust-overlay";
      rev = "a35d1c25f95de9b6cf1f2bff3bc9f81ada84e776";
      sha256 = "sha256-YgEOZT7i0yV0X6e1/+XQlPT3eWQ2GpqhgRBAKdJSq7c=";
    });
  };
in
outputs.packages.${system}.default
