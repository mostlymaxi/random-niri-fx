{
  description = "random-fx: a thin line that lives along the edges of the screen";

  # Only nixpkgs, so consumers can `inputs.nixpkgs.follows = "nixpkgs"`.
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      overlays.default = final: _prev: {
        random-fx = final.callPackage ./package.nix { };
      };

      packages = forAllSystems (pkgs: rec {
        random-fx = pkgs.callPackage ./package.nix { };
        default = random-fx;
      });

      homeModules.default = import ./nix/hm-module.nix self;

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.random-fx ];
          packages = with pkgs; [
            rustfmt
            clippy
            pre-commit
          ];
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
        };
      });

      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}
