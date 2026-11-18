{
  description = "Development environment for ELA Local";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forEachSystem = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forEachSystem (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            # mkShell also supplies the platform's C compiler for bundled SQLite.
            packages =
              (with pkgs; [
                cargo
                rustc
                rustfmt
                clippy
                rust-analyzer
                nodejs
                prettier
                python3
                ruff
                ffmpeg-headless
                cmake
                (tesseract.override {
                  enableLanguages = [
                    "eng"
                    "tel"
                  ];
                })
              ])
              ++ pkgs.lib.optionals (system != "aarch64-linux") [ pkgs.udpipe1 ];

            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
            # Use whisper-rs-sys' shipped FFI bindings; no host libclang needed.
            WHISPER_DONT_GENERATE_BINDINGS = "1";
            CMAKE_BUILD_PARALLEL_LEVEL = "4";
          };
        }
      );

      formatter = forEachSystem (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        pkgs.writeShellApplication {
          name = "format-nix";
          runtimeInputs = [ pkgs.nixfmt ];
          text = ''
            if [ "$#" -eq 0 ]; then
              set -- flake.nix
            fi
            exec nixfmt "$@"
          '';
        }
      );
    };
}
