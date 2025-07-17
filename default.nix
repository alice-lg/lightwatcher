{ pkgs ? import <nixpkgs> {} }:

pkgs.rustPlatform.buildRustPackage rec {
  pname = "lightwatcher";
  version = "3.0.0";
  
  src = ./.;
  
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
  
  meta = with pkgs.lib; {
    description = "A lightweight clone of birdwatcher";
    homepage = "https://github.com/alice-lg/lightwatcher";
  };
}
