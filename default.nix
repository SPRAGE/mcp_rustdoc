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
    fenix = import (pkgs.fetchFromGitHub {
      owner = "nix-community";
      repo = "fenix";
      rev = "ebaf9f5fd6f15685091c2181a5b685120e2606f5";
      sha256 = "sha256-Yf3v730dtGhrGNtdlwnyBEr9kCXSEh1pS8TFfpCahJY=";
    });
  };
in
outputs.packages.${system}.default
