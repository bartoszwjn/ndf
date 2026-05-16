{
  lib,
  craneLib,
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

  deny = craneLib.cargoDeny {
    inherit (baseArgs) src strictDeps;
    cargoDenyChecks = "bans licenses sources";
    cargoDenyExtraArgs = "--workspace";
  };

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

  test = craneLib.cargoTest (
    commonArgs
    // {
      # Skip integration tests, since those need to be able to execute Nix commands,
      # which is hard to do inside a Nix build sandbox.
      cargoTestExtraArgs = "--workspace --lib --bins --examples";
    }
  );
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
        deny
        doc
        fmt
        test
        # keep-sorted end
        ;
    };
  }
)
