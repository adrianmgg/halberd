{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
        lib = pkgs.lib;
        craneLib = crane.mkLib pkgs;
        src = lib.cleanSourceWith {
          src = lib.cleanSource ./.;
          filter = orig_path: type:
            craneLib.filterCargoSources orig_path type;
          name = "sources";
        };
        commonArgs = {
          inherit src;
          strictDeps = true;
          buildInputs = [];
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        halberd = craneLib.buildPackage (commonArgs // {inherit cargoArtifacts;});
      in {
        checks = {
          inherit halberd;
          halberd-clippy = craneLib.cargoClippy (commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            });
          halberd-doc = craneLib.cargoDoc (commonArgs // {inherit cargoArtifacts;});
          halberd-fmt = craneLib.cargoFmt {inherit src;};
        };
        packages.default = halberd;
        apps.default = flake-utils.lib.mkApp {
          drv = halberd;
        };
        devShells.default = craneLib.devShell {
          # checks = self.checks.${system};
          packages = [];
        };
      }
    );
}
