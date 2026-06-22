{
  description = "Minimalist developer sandbox and disposable VM manager for UTM on macOS.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "aarch64-darwin";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      packages.aarch64-darwin.default = pkgs.rustPlatform.buildRustPackage {
        pname = "utmd";
        version = "0.1.2";
        src = ./.;

        cargoLock = {
          lockFile = ./Cargo.lock;
        };

        nativeBuildInputs = with pkgs; [ installShellFiles ];

        meta = with pkgs.lib; {
          description = "Minimalist developer sandbox and disposable VM manager for UTM on macOS.";
          homepage = "https://github.com/tappunk/utmd";
          license = licenses.mit;
          maintainers = [ ];
          platforms = [ "aarch64-darwin" ];
          mainProgram = "utmd";
        };
      };

      devShells.aarch64-darwin.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          rustc
          cargo
          clippy
          rustfmt
          rust-analyzer
        ];

        shellHook = ''
          echo "utmd dev environment loaded (aarch64-darwin)"
        '';
      };

      apps.aarch64-darwin.default = {
        type = "app";
        program = "${self.packages.aarch64-darwin.default}/bin/utmd";
      };
    };
}
