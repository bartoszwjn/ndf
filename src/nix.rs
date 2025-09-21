use std::{path::Path, process::Command};

use crate::{
    command,
    items::{GitRev, Item, SourceType},
};

fn get_current_system() -> anyhow::Result<String> {
    command::run_json(
        "nix-instantiate",
        &["--eval", "--json", "--expr", "builtins.currentSystem"],
    )
}

pub(crate) fn get_current_flake_packages() -> anyhow::Result<Vec<String>> {
    let current_system = get_current_system()?;
    let package_names_fn =
        format!("flake: builtins.attrNames ((flake.packages or {{}}).{current_system} or {{}})");
    command::run_json(
        "nix",
        &["eval", "--json", ".#.", "--apply", &package_names_fn],
    )
}

pub(crate) fn get_current_flake_nixos_configurations() -> anyhow::Result<Vec<String>> {
    let nixos_names_fn = "flake: builtins.attrNames (flake.nixosConfigurations or {})";
    command::run_json("nix", &["eval", "--json", ".#.", "--apply", nixos_names_fn])
}

pub(crate) fn get_file_output_attributes(file: &Path) -> anyhow::Result<Vec<String>> {
    command::output_json(
        Command::new("nix")
            .args(["eval", "--json", "--file"])
            .arg(file)
            .args([
                "--apply",
                "x: let r = if builtins.isFunction x then x {} else x; in builtins.attrNames r",
            ]),
    )
}

pub(crate) fn get_drv_path(item: &Item) -> anyhow::Result<String> {
    match item.source {
        SourceType::FlakeCurrentDir => {
            let flake_ref = match &item.git_rev {
                GitRev::Worktree => String::from(".#"),
                GitRev::Rev { rev, .. } => format!(".?rev={rev}#"),
            } + &item.attr_path;
            command::run_json(
                "nix",
                &[
                    "eval",
                    "--json",
                    "--apply",
                    "v: v.drvPath",
                    "--",
                    &flake_ref,
                ],
            )
        }
        SourceType::File(_) => todo!("get drv path from file"),
    }
}
