use std::path::Path;

use eyre::WrapErr;

use crate::{
    attr_path::AttrPath,
    command::Cmd,
    diff_spec::{FlakePath, Source},
};

fn get_current_system() -> eyre::Result<String> {
    Cmd::nix()
        .args(["--extra-experimental-features", "nix-command"])
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
    let flake_ref = format!("{}#.", flake_path.as_str());
    Cmd::nix()
        .args(["--extra-experimental-features", "nix-command flakes"])
        .args(["eval", "--json", "--apply", &package_names_fn, "--"])
        .arg(flake_ref)
        .output_json()
}

pub(crate) fn get_flake_nixos_configurations(flake_path: &FlakePath) -> eyre::Result<Vec<String>> {
    let nixos_names_fn = "flake: builtins.attrNames (flake.nixosConfigurations or {})";
    let flake_ref = format!("{}#.", flake_path.as_str());
    Cmd::nix()
        .args(["--extra-experimental-features", "nix-command flakes"])
        .args(["eval", "--json", "--apply", nixos_names_fn, "--"])
        .arg(flake_ref)
        .output_json()
}

pub(crate) fn get_file_output_attributes(file: &Path) -> eyre::Result<Vec<String>> {
    let attr_names_fn = "x: builtins.attrNames (if builtins.isFunction x then x {} else x)";
    Cmd::nix()
        .args(["--extra-experimental-features", "nix-command"])
        .args(["eval", "--json", "--apply", attr_names_fn, "--file"])
        .arg(file)
        .output_json()
}

pub(crate) fn get_drv_path(
    repo_root: &Path,
    source: &Source,
    commit_id: Option<&str>,
    attr_path: &AttrPath,
) -> eyre::Result<String> {
    let mut cmd = Cmd::nix();
    match source {
        Source::Flake(_) => {
            cmd.args(["--extra-experimental-features", "nix-command flakes"]);
        }
        Source::File(_) => {
            cmd.args(["--extra-experimental-features", "nix-command"]);
        }
    }
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
            let fragment = attr_path.to_flake_fragment().wrap_err_with(|| {
                format!("failed to evaluate derivation path of flake output {attr_path:?}")
            })?;
            let flake_ref = if let Some(rev) = commit_id {
                format!("{}?rev={}#{}", flake_path.as_str(), rev, fragment)
            } else {
                format!("{}#{}", flake_path.as_str(), fragment)
            };

            cmd.args(["--", &flake_ref]).output_json()
        }
        Source::File(path) => {
            let attribute = attr_path
                .to_cli_arg()
                .wrap_err_with(|| {
                    format!("failed to evaluate derivation path of attribute {attr_path:?}")
                })?
                .to_string();
            match commit_id {
                None => cmd
                    .arg("--file")
                    .arg(path)
                    .args(["--", &attribute])
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
                        .args(["--", &attribute])
                        .output_json()
                }
            }
        }
    }
}
