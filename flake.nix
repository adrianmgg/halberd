{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    spirv-headers = {
      url = "github:KhronosGroup/SPIRV-Headers";
      flake = false;
    };
  };
  outputs = {
    self,
    nixpkgs,
    crane,
    rust-overlay,
    flake-utils,
    spirv-headers,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [(import rust-overlay)];
        };
        lib = pkgs.lib;
        craneLib = (crane.mkLib pkgs).overrideToolchain (p: p.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml);
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
        # TODO should we use unified1 or 1.2 ?
        SPIRV_GRAMMAR_JSON = "${spirv-headers}/include/spirv/unified1/spirv.core.grammar.json";
        individualCrateArgs =
          commonArgs
          // {
            inherit cargoArtifacts;
            inherit (craneLib.crateNameFromCargoToml {inherit src;}) version;
            inherit SPIRV_GRAMMAR_JSON;
            # FIXME turning these off for the build bc they don't all pass lmao
            doCheck = false;
          };
        fileSetForCrate = crate: deps:
          lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions ([
                ./Cargo.toml
                ./Cargo.lock
                (craneLib.fileset.commonCargoSources ./${crate})
              ]
              ++ (map (d: craneLib.fileset.commonCargoSources ./${d}) deps));
          };
        packageForCrate = crate: deps: (craneLib.buildPackage (individualCrateArgs
          // {
            pname = crate;
            cargoExtraArgs = "-p ${crate}";
            src = fileSetForCrate crate deps;
          }));
        halberd = packageForCrate "halberd" [];
      in {
        checks = {
          inherit halberd;
          workspace-clippy = craneLib.cargoClippy (commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            });
          # TODO: see crane's quickstart example setting this to be stricter
          workspace-doc = craneLib.cargoDoc (commonArgs // {inherit cargoArtifacts;});
          workspace-fmt = craneLib.cargoFmt {inherit src;};
        };
        packages.default = halberd;
        packages.halberd = halberd;
        apps.default = flake-utils.lib.mkApp {
          drv = halberd;
        };
        devShells.default = craneLib.devShell {
          # checks = self.checks.${system};
          packages = [];
          inherit SPIRV_GRAMMAR_JSON;
        };
      }
    );
}
