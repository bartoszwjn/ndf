use std::{
    collections::BTreeMap,
    fs,
    io::ErrorKind,
    path::Path,
    process::{Command, Stdio},
};

use crate::common;

/// Manage example Nix projects used for testing.
#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[command(subcommand)]
    subcommand: Subcommand,
}

impl Args {
    pub(crate) fn exec(self) -> eyre::Result<()> {
        match self.subcommand {
            Subcommand::Make(make_args) => make_args.exec(),
            Subcommand::Clean(clean_args) => clean_args.exec(),
        }
    }
}

#[derive(clap::Subcommand, Debug)]
enum Subcommand {
    Make(MakeArgs),
    Clean(CleanArgs),
}

/// Generate example directories.
#[derive(clap::Args, Debug)]
struct MakeArgs {
    /// Generate only these examples.
    #[arg(value_name = "NAME")]
    names: Option<Vec<String>>,
}

impl MakeArgs {
    fn exec(self) -> eyre::Result<()> {
        let system = get_current_system()?;
        let examples = examples(&system);

        let names = match self.names {
            Some(mut names) => {
                names.sort();
                names.dedup();
                names
            }
            None => examples.keys().copied().map(String::from).collect(),
        };

        for name in names {
            examples
                .get(name.as_str())
                .ok_or_else(|| eyre::eyre!("undefined example: {name:?}"))?
                .make()?;
        }

        Ok(())
    }
}

/// Remove generated example directories.
#[derive(clap::Args, Debug)]
struct CleanArgs {
    /// Clean up only these examples.
    #[arg(value_name = "NAME")]
    names: Option<Vec<String>>,
}

impl CleanArgs {
    fn exec(self) -> eyre::Result<()> {
        let mut examples_dir = common::workspace_root().join("examples");
        match self.names {
            None => remove_dir(&examples_dir),
            Some(names) => {
                for name in &names {
                    examples_dir.push(name);
                    remove_dir(&examples_dir)?;
                    examples_dir.pop();
                }
                Ok(())
            }
        }
    }
}

struct Example<'a> {
    name: &'static str,
    #[allow(clippy::type_complexity)]
    ops: Box<dyn Fn(&Path) -> eyre::Result<()> + 'a>,
}

impl Example<'_> {
    fn make(&self) -> eyre::Result<()> {
        let dir = common::workspace_root().join("examples").join(self.name);
        init_repo(&dir)?;
        (self.ops)(&dir)?;
        Ok(())
    }
}

fn examples(system: &str) -> BTreeMap<&'static str, Example<'_>> {
    [
        Example {
            name: "clean",
            ops: Box::new(|dir| {
                let packages1 = [r#"pkg1 = drv "pkg1-1""#, r#"pkg2 = drv "pkg2""#];
                let packages2 = [r#"pkg1 = drv "pkg1-2""#, r#"pkg2 = drv "pkg2""#];

                write_drv_nix(dir, system)?;
                write_package_flake_nix(dir, system, packages1)?;
                write_package_default_nix(dir, packages1)?;
                commit(dir, "commit 1")?;
                write_package_flake_nix(dir, system, packages2)?;
                write_package_default_nix(dir, packages2)?;
                commit(dir, "commit 2")?;

                Ok(())
            }),
        },
        Example {
            name: "dirty",
            ops: Box::new(|dir| {
                let packages1 = [r#"pkg1 = drv "pkg1-1""#, r#"pkg2 = drv "pkg2""#];
                let packages2 = [r#"pkg1 = drv "pkg1-2""#, r#"pkg2 = drv "pkg2""#];

                write_drv_nix(dir, system)?;
                write_package_flake_nix(dir, system, packages1)?;
                write_package_default_nix(dir, packages1)?;
                commit(dir, "commit 1")?;
                write_package_flake_nix(dir, system, packages2)?;
                write_package_default_nix(dir, packages2)?;

                Ok(())
            }),
        },
        Example {
            name: "function-file",
            ops: Box::new(|dir| {
                let packages1 = [r#"pkg1 = drv "pkg1-${arg1}""#, r#"pkg2 = drv "pkg2""#];
                let packages2 = [r#"pkg1 = drv "pkg1-${arg1}""#, r#"pkg2 = drv "pkg2""#];

                let expr1 = r#"{ arg1 ? "1" }: "#.to_owned()
                    + &default_nix_expr("drv = import ./drv.nix", packages1);
                let expr2 = r#"{ arg1 ? "2" }: "#.to_owned()
                    + &default_nix_expr("drv = import ./drv.nix", packages2);

                write_drv_nix(dir, system)?;
                write_file(dir, "default.nix", &expr1)?;
                commit(dir, "commit 1")?;
                write_file(dir, "default.nix", &expr2)?;
                commit(dir, "commit 2")?;

                Ok(())
            }),
        },
        Example {
            name: "empty-repo",
            ops: Box::new(|dir| {
                commit(dir, "commit 1")?;
                commit(dir, "commit 2")?;
                Ok(())
            }),
        },
        Example {
            name: "no-outputs",
            ops: Box::new(|dir| {
                write_file(dir, "flake.nix", "{ outputs = inputs: {}; }\n")?;
                write_file(dir, "default.nix", "{}\n")?;
                commit(dir, "commit 1")?;
                commit(dir, "commit 2")?;
                Ok(())
            }),
        },
        Example {
            name: "different-outputs",
            ops: Box::new(|dir| {
                let packages1 = [r#"pkg1 = drv "pkg1-1""#, r#"pkg2 = drv "pkg2""#];
                let packages2 = [r#"pkg1 = drv "pkg1-2""#, r#"pkg3 = drv "pkg3""#];

                write_drv_nix(dir, system)?;
                write_package_flake_nix(dir, system, packages1)?;
                write_package_default_nix(dir, packages1)?;
                commit(dir, "commit 1")?;
                write_package_flake_nix(dir, system, packages2)?;
                write_package_default_nix(dir, packages2)?;
                commit(dir, "commit 2")?;

                Ok(())
            }),
        },
        Example {
            name: "nested",
            ops: Box::new(|dir| {
                let packages1 = [r#"a.b.pkg1 = drv "pkg1-1""#, r#"a.b.pkg2 = drv "pkg2""#];
                let packages2 = [r#"a.b.pkg1 = drv "pkg1-2""#, r#"a.b.pkg2 = drv "pkg2""#];

                write_drv_nix(dir, system)?;
                write_package_flake_nix(dir, system, packages1)?;
                write_package_default_nix(dir, packages1)?;
                commit(dir, "commit 1")?;
                write_package_flake_nix(dir, system, packages2)?;
                write_package_default_nix(dir, packages2)?;
                commit(dir, "commit 2")?;

                Ok(())
            }),
        },
        Example {
            name: "nixos",
            ops: Box::new(|dir| {
                let cfgs1 = [r#"nixos1 = nixos "nixos1-1""#, r#"nixos2 = nixos "nixos2""#];
                let cfgs2 = [r#"nixos1 = nixos "nixos1-2""#, r#"nixos2 = nixos "nixos2""#];

                write_nixos_nix(dir, system)?;
                write_nixos_flake_nix(dir, cfgs1)?;
                write_nixos_default_nix(dir, cfgs1)?;
                commit(dir, "commit 1")?;
                write_nixos_flake_nix(dir, cfgs2)?;
                write_nixos_default_nix(dir, cfgs2)?;
                commit(dir, "commit 2")?;

                Ok(())
            }),
        },
    ]
    .into_iter()
    .map(|example| (example.name, example))
    .collect()
}

fn get_current_system() -> eyre::Result<String> {
    let output = Command::new("nix")
        .args([
            "eval",
            "--impure",
            "--raw",
            "--expr",
            "builtins.currentSystem",
        ])
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        eyre::bail!("command failed ({})", output.status);
    }

    Ok(String::from_utf8(output.stdout)?)
}

fn write_package_flake_nix(
    dir: &Path,
    system: &str,
    packages: impl IntoIterator<Item = impl AsRef<str>>,
) -> eyre::Result<()> {
    let contents = flake_nix_expr(
        "drv = import ./drv.nix",
        &format!("packages.{system}"),
        packages,
    );
    write_file(dir, "flake.nix", &contents)
}

fn write_nixos_flake_nix(
    dir: &Path,
    configurations: impl IntoIterator<Item = impl AsRef<str>>,
) -> eyre::Result<()> {
    let contents = flake_nix_expr(
        "nixos = import ./nixos.nix",
        "nixosConfigurations",
        configurations,
    );
    write_file(dir, "flake.nix", &contents)
}

fn flake_nix_expr(
    import: &str,
    attr: &str,
    outputs: impl IntoIterator<Item = impl AsRef<str>>,
) -> String {
    let mut expr = format!("{{ outputs = inputs: let {import}; in {{ {attr} = {{\n");
    for output in outputs {
        expr = expr + "  " + output.as_ref() + ";\n";
    }
    expr.push_str("}; }; }\n");
    expr
}

fn write_package_default_nix(
    dir: &Path,
    packages: impl IntoIterator<Item = impl AsRef<str>>,
) -> eyre::Result<()> {
    let contents = default_nix_expr("drv = import ./drv.nix", packages);
    write_file(dir, "default.nix", &contents)
}

fn write_nixos_default_nix(
    dir: &Path,
    configurations: impl IntoIterator<Item = impl AsRef<str>>,
) -> eyre::Result<()> {
    let contents = default_nix_expr("nixos = import ./nixos.nix", configurations);
    write_file(dir, "default.nix", &contents)
}

fn default_nix_expr(import: &str, outputs: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    let mut expr = format!("let {import}; in {{\n");
    for output in outputs {
        expr = expr + "  " + output.as_ref() + ";\n";
    }
    expr.push_str("}\n");
    expr
}

fn write_drv_nix(dir: &Path, system: &str) -> eyre::Result<()> {
    let drv_expr = format!("name: {}\n", derivation_expr(system));
    write_file(dir, "drv.nix", &drv_expr)
}

fn write_nixos_nix(dir: &Path, system: &str) -> eyre::Result<()> {
    let nixos_expr = format!(
        "name: {{ config.system.build.toplevel = {}; }}\n",
        derivation_expr(system),
    );
    write_file(dir, "nixos.nix", &nixos_expr)
}

fn derivation_expr(system: &str) -> String {
    format!(
        "derivation {{ \
            inherit name; \
            system = \"{system}\"; \
            builder = builtins.toFile \"builder.sh\" ''\n\
                #!/bin/sh\n\
                echo 'This derivation cannot be built!'\n\
                exit 1\n\
            ''; \
        }}"
    )
}

fn write_file(dir: &Path, file_path: impl AsRef<Path>, contents: &str) -> eyre::Result<()> {
    let file_path = file_path.as_ref();
    assert!(file_path.is_relative());
    let path = dir.join(file_path);

    eprintln!("creating {path:?}");
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, contents)?;
    Ok(())
}

fn init_repo(dir: &Path) -> eyre::Result<()> {
    remove_dir(dir)?;
    fs::create_dir_all(dir)?;

    common::run(Command::new("git").current_dir(dir).arg("init"))?;
    commit(dir, "init")?;

    Ok(())
}

fn commit(dir: &Path, message: &str) -> eyre::Result<()> {
    common::run(Command::new("git").current_dir(dir).args(["add", "--all"]))?;
    common::run(Command::new("git").current_dir(dir).args([
        "commit",
        "--allow-empty",
        "--message",
        message,
    ]))?;
    Ok(())
}

fn remove_dir(path: &Path) -> eyre::Result<()> {
    eprintln!("removing {path:?}");
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if let ErrorKind::NotFound = error.kind() => Ok(()),
        Err(error) => Err(error.into()),
    }
}
