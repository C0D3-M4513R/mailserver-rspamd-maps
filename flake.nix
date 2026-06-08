{
  inputs = {
    # This must be the stable nixpkgs if you're running the app on a
    # stable NixOS install.  Mixing EGL library versions doesn't work.
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, utils, rust-overlay, ... }:
    utils.lib.eachSystem ["x86_64-linux" "aarch64-linux"] (system:
      let

        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
            inherit system overlays;
        };

        rustVersion = pkgs.rust-bin.stable.latest.default;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustVersion;
          rustc = rustVersion;
        };

        manifest = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;

        commonBuildInputs = with pkgs; [
          pkg-config
        ];

        package = pkgs.rust.packages.stable.rustPlatform.buildRustPackage rec{
          pname = manifest.name;
          version = manifest.version;
          src = pkgs.lib.cleanSource ./.;
          cargoBuildFlags = "";

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [
            pkgs.autoPatchelfHook
          ];

          buildInputs = with pkgs; [
            pkgs.rust-bin.stable.latest.default
          ] ++ commonBuildInputs;

          # Certain Rust tools won't work without this
          # This can also be fixed by using oxalica/rust-overlay and specifying the rust-src extension
          # See https://discourse.nixos.org/t/rust-src-not-found-and-other-misadventures-of-developing-rust-on-nixos/11570/3?u=samuela. for more details.
          RUST_SRC_PATH = pkgs.rust.packages.stable.rustPlatform.rustLibSrc;
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          #LD_LIBRARY_PATH = libPath;
          OPENSSL_LIB_DIR = pkgs.openssl.out + "/lib";

          meta = {
#            description = "A Discord Bot";
#            homepage = "https://github.com/Me-n-the-Boys/MeAndTheBoysBot";
#            license = nixpkgs.lib.licenses.unfree; #This repo has no license and should be taken as All-Rights-Reserved.
            maintainers = [];
            mainProgram = "mailserver-rspamd-maps";
          };
        };
      in
      {
        packages = {
        		default = package;
            "${manifest.name}" = package;
        };

        defaultApp = utils.lib.mkApp {
          drv = self.defaultPackage."${system}";
        };

        devShell = with pkgs; mkShell {
          buildInputs = [
            #cargo
            cargo-insta
            pre-commit
            #rust-analyzer
            #rustPackages.clippy
            #rustc
            #rustfmt
            tokei
        (
            python312.withPackages (ps: with ps; [
                requests
                colorama
                aiohttp
            ])
        )
          ] ++ commonBuildInputs;
          RUST_SRC_PATH = pkgs.rust.packages.stable.rustPlatform.rustLibSrc;
          LD_LIBRARY_PATH = lib.makeLibraryPath commonBuildInputs;
          GIT_EXTERNAL_DIFF = "${difftastic}/bin/difft";
          RUST_BACKTRACE= "1";
          RUST_LIB_BACKTRACE = "1";
        };
      });
}
