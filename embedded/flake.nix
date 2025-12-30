{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, naersk }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
      in
      {
        defaultPackage = naersk-lib.buildPackage ./.;
        devShell = with pkgs; mkShell {
          buildInputs = [ cargo rustc rustfmt pre-commit rustPackages.clippy picotool probe-rs-tools elf2uf2-rs nixfmt ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
          shellHook = ''
            export PATH="$HOME/.cargo/bin:$PATH"
            rustup default nightly
            rustup target add thumbv6m-none-eabi
          '';
        };
      }
    );
}
