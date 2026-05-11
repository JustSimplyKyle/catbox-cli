{
  description = "catbox-cli
cbx";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, utils, rust-overlay }:
    utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "catbox-cli";
          version = "0.1.0";

          src = ./.;

          cargoHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [ 
            rustToolchain
            pkgs.cargo 
            pkgs.rustc 
            pkgs.dbus
            pkgs.openssl
            pkgs.rust-analyzer 
            pkgs.pkg-config
          ];
      
          # Helpful for tools that rely on dynamic linking (like OpenCV)
          # LD_LIBRARY_PATH = "\/nix/store/n0flbxn0w105xr0d95rj605llxg6812f-opencv-4.13.0/lib";
        };
      });
}
