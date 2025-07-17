{ pkgs ? import <nixpkgs> {} }:

pkgs.rustPlatform.buildRustPackage rec {
  pname = "lightwatcher";
  
  src = ./.;
  
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
  
  meta = with pkgs.lib; {
    description = "A lightweight clone of birdwatcher";
    homepage = "https://github.com/alice-lg/lightwatcher";
  };
}
