use std::path::Path;

use crate::{
    command::Cmd,
    spec::{AttrPath, GitRev, Source},
};

fn get_current_system() -> anyhow::Result<String> {
    Cmd::nix()
        .args([
            "eval",
            "--impure",
            "--json",
            "--expr",
            "builtins.currentSystem",
        ])
        .output_json()
}

pub(crate) fn get_current_flake_packages() -> anyhow::Result<Vec<String>> {
    let current_system = get_current_system()?;
    let package_names_fn =
        format!("flake: builtins.attrNames (flake.packages.{current_system} or {{}})");
    Cmd::nix()
        .args(["eval", "--json", ".#.", "--apply", &package_names_fn])
        .output_json()
}

pub(crate) fn get_current_flake_nixos_configurations() -> anyhow::Result<Vec<String>> {
    let nixos_names_fn = "flake: builtins.attrNames (flake.nixosConfigurations or {})";
    Cmd::nix()
        .args(["eval", "--json", ".#.", "--apply", nixos_names_fn])
        .output_json()
}

pub(crate) fn get_file_output_attributes(file: &Path) -> anyhow::Result<Vec<String>> {
    Cmd::nix()
        .args(["eval", "--json", "--file"])
        .arg(file)
        .args([
            "--apply",
            "x: let r = if builtins.isFunction x then x {} else x; in builtins.attrNames r",
        ])
        .output_json()
}

pub(crate) fn get_drv_path(
    source: &Source,
    git_rev: &GitRev,
    attr_path: &AttrPath,
) -> anyhow::Result<String> {
    match source {
        Source::FlakeCurrentDir => {
            let flake_ref = match git_rev {
                GitRev::Worktree => String::from(".#"),
                GitRev::Rev { rev, .. } => format!(".?rev={rev}#"),
            } + &attr_path.0;
            Cmd::nix()
                .args([
                    "eval",
                    // Eval cache hardly works, sometimes it even seems to make things slower.
                    // It also causes Nix to report "SQLite database is busy" errors
                    // when running multiple evaluations in parallel.
                    "--no-eval-cache",
                    "--json",
                    "--apply",
                    "v: v.drvPath",
                    "--",
                    &flake_ref,
                ])
                .output_json()
        }
        Source::File(_) => todo!("get drv path from file"),
    }
}
