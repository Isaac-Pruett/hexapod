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

        # IMPORTANT: naersk must be wired to the toolchain you want
        # (don’t let it default to pkgs.cargo/pkgs.rustc)
        rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
          targets = [
            "thumbv8m.main-none-eabihf"
            "thumbv8m.main-none-eabi"
          ];
          extensions = [ "rust-src" "rustfmt" "clippy" ];
        };

        naersk-lib = pkgs.callPackage naersk {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        arch = "thumbv8m.main-none-eabihf";
      in
      {
        packages.default = naersk-lib.buildPackage {
          pname = "embedded";
          src = ./.;
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustToolchain   # <- this provides cargo/rustc/rustfmt/clippy + targets
            pre-commit
            picotool
            probe-rs-tools
            elf2uf2-rs
            nixfmt
          ];

          CARGO_BUILD_TARGET = arch;

          shellHook = ''
            shellinfo() {
              echo "Rust:  $(rustc --version)"
              echo "Cargo: $(cargo --version)"
              echo "Targets available (filtered):"
              rustc --print target-list | grep thumbv8m || true
              echo "Installed rust-std components:"
              rustc --print sysroot || true
            }
          '';
        };

        apps.default = utils.lib.mkApp { drv = self.packages.${system}.default; };
      }
    );
}
