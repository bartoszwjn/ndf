use std::path::Path;

use crate::{
    command::Cmd,
    diff_spec::{AttrPath, FlakePath, Source},
};

fn get_current_system() -> eyre::Result<String> {
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

pub(crate) fn get_flake_packages(flake_path: &FlakePath) -> eyre::Result<Vec<String>> {
    let current_system = get_current_system()?;
    let package_names_fn =
        format!("flake: builtins.attrNames (flake.packages.{current_system} or {{}})");
    let mut flake_ref = flake_path.path().as_os_str().to_os_string();
    flake_ref.push("#.");
    Cmd::nix()
        .args(["eval", "--json", "--apply", &package_names_fn, "--"])
        .arg(flake_ref)
        .output_json()
}

pub(crate) fn get_flake_nixos_configurations(flake_path: &FlakePath) -> eyre::Result<Vec<String>> {
    let nixos_names_fn = "flake: builtins.attrNames (flake.nixosConfigurations or {})";
    let mut flake_ref = flake_path.path().as_os_str().to_os_string();
    flake_ref.push("#.");
    Cmd::nix()
        .args(["eval", "--json", "--apply", nixos_names_fn, "--"])
        .arg(flake_ref)
        .output_json()
}

pub(crate) fn get_file_output_attributes(file: &Path) -> eyre::Result<Vec<String>> {
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
    repo_root: &Path,
    source: &Source,
    commit_id: Option<&str>,
    attr_path: &AttrPath,
) -> eyre::Result<String> {
    let mut cmd = Cmd::nix();
    cmd.args([
        "eval",
        // Eval cache hardly works, sometimes it even seems to make things slower.
        // It also causes Nix to report "SQLite database is busy" errors
        // when running multiple evaluations in parallel.
        "--no-eval-cache",
        "--json",
        "--apply",
        "v: v.drvPath",
    ]);

    match source {
        Source::Flake(flake_path) => {
            let mut flake_ref = flake_path.path().as_os_str().to_os_string();
            if let Some(rev) = commit_id {
                flake_ref.push("?rev=");
                flake_ref.push(rev);
            }
            flake_ref.push("#");
            flake_ref.push(&attr_path.0);

            cmd.arg("--").arg(&flake_ref).output_json()
        }
        Source::File(path) => match commit_id {
            None => cmd
                .arg("--file")
                .arg(path)
                .args(["--", &attr_path.0])
                .output_json(),
            Some(rev) => {
                let Ok(path_relative) = path.strip_prefix(repo_root) else {
                    unreachable!("repo_root should be a prefix of the file path");
                };

                const EXPR: &str = "{repoRoot, pathInRepo, rev}: \
                    let \
                      repo = builtins.fetchGit { url = /. + repoRoot; inherit rev; }; \
                      path = if pathInRepo == \"\" then repo else repo + \"/${pathInRepo}\"; \
                      autoApply = x: if builtins.isFunction x then x {} else x; \
                    in \
                    autoApply (import path)";

                cmd.args(["--impure", "--expr", EXPR])
                    .args(["--argstr", "repoRoot"])
                    .arg(repo_root)
                    .args(["--argstr", "pathInRepo"])
                    .arg(path_relative)
                    .args(["--argstr", "rev", rev])
                    .args(["--", &attr_path.0])
                    .output_json()
            }
        },
    }
}
