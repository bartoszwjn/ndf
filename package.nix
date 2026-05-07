{
  lib,
  craneLib,
  cargo-hakari,
}:

let
  src = craneLib.cleanCargoSource ./.;
  cargoToml = lib.importTOML ./Cargo.toml;

  baseArgs = {
    inherit src;
    strictDeps = true;
  };
  commonArgs = baseArgs // {
    inherit cargoArtifacts;
  };

  cargoArtifacts = craneLib.buildDepsOnly baseArgs;

  clippy = craneLib.cargoClippy (
    commonArgs // { cargoClippyExtraArgs = "--workspace --all-targets -- --deny warnings"; }
  );

  doc = craneLib.cargoDoc (
    commonArgs
    // {
      cargoDocExtraArgs = "--no-deps --workspace";
      env.RUSTDOCFLAGS = "--deny warnings";
    }
  );

  fmt = craneLib.cargoFmt {
    inherit (baseArgs) src strictDeps;
    cargoExtraArgs = "--all"; # `--workspace` equivalent
  };

  hakari = craneLib.mkCargoDerivation {
    inherit (baseArgs) src strictDeps;

    pname = "${cargoToml.package.name}-hakari";
    cargoArtifacts = null;
    doInstallCargoArtifacts = false;

    nativeBuildInputs = [ cargo-hakari ];

    buildPhaseCargoCommand = ''
      cargo hakari generate --diff
      cargo hakari manage-deps --dry-run
      cargo hakari verify
    '';
  };

  test = craneLib.cargoTest (commonArgs // { cargoTestExtraArgs = "--workspace"; });
in

craneLib.buildPackage (
  commonArgs
  // {
    doCheck = false;

    meta = {
      description = cargoToml.package.description;
      homepage = cargoToml.package.homepage or cargoToml.package.repository;
      license =
        assert cargoToml.package.license == "MIT OR Apache-2.0";
        [
          lib.licenses.mit
          lib.licenses.asl20
        ];
      mainProgram = cargoToml.package.default-run;
    };

    passthru.tests = {
      inherit
        # keep-sorted start
        clippy
        doc
        fmt
        hakari
        test
        # keep-sorted end
        ;
    };
  }
)
