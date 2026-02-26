{
  description = "Tool for comparing multiple Nix derivations between commits";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default";
    crane.url = "github:ipetkov/crane";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    let
      inherit (inputs.nixpkgs) lib;

      eachSystem =
        f:
        lib.pipe (import inputs.systems) [
          (map (system: lib.mapAttrs (k: v: { ${system} = v; }) (f system)))
          (lib.zipAttrsWith (k: lib.foldl' (acc: attrs: acc // attrs) { }))
        ];
    in
    eachSystem (
      system:
      let
        pkgs = inputs.nixpkgs.legacyPackages.${system};
        craneLib = import inputs.crane { inherit pkgs; };

        treefmtEval = (import inputs.treefmt-nix).evalModule pkgs {
          projectRootFile = "flake.nix";
          settings.on-unmatched = "info";
          programs.nixfmt.enable = true;
          programs.keep-sorted.enable = true;
          programs.rustfmt.enable = true;
        };

        ndf = import ./package.nix { inherit lib craneLib; };
      in
      {
        packages = {
          inherit ndf;
          default = ndf;
        };

        checks = {
          inherit ndf;
          treefmt-check = treefmtEval.config.build.check (
            lib.fileset.toSource {
              root = ./.;
              fileset = lib.fileset.gitTracked ./.;
            }
          );
        }
        // lib.mapAttrs' (testName: lib.nameValuePair "ndf-test-${testName}") ndf.tests;

        devShells.default = craneLib.devShell {
          inputsFrom = [ treefmtEval.config.build.devShell ];
        };

        formatter = treefmtEval.config.build.wrapper;
      }
    );
}
