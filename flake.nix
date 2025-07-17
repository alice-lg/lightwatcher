{
  description = "Lightwatcher - A lightweight replacement for birdwatcher";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.callPackage ./default.nix { };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            rustfmt
            rust-analyzer
            clippy
            
            # Dev tools
            cargo-watch
            bacon
            
            # For testing with BIRD
            bird3
          ];
          
          RUST_LOG = "info";
          RUST_BACKTRACE = 1;
        };
      });
}
