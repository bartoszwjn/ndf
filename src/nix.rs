use std::path::Path;

use eyre::WrapErr;

use crate::{
    attr_path::AttrPath,
    command::Cmd,
    diff_spec::{FlakePath, Source},
};

fn get_current_system() -> eyre::Result<String> {
    Cmd::nix_instantiate()
        .args(["--eval", "--strict", "--json"])
        .args(["--expr", "builtins.currentSystem"])
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
    Cmd::nix_instantiate()
        .args(["--eval", "--strict", "--json"])
        .args(["--expr", include_str!("nix/get-file-output-attributes.nix")])
        .args(["--argstr", "path"])
        .arg(file)
        .output_json()
}

pub(crate) fn get_drv_path(
    repo_root: &Path,
    source: &Source,
    commit_id: Option<&str>,
    attr_path: &AttrPath,
) -> eyre::Result<String> {
    match source {
        Source::Flake(flake_path) => get_drv_path_flake(flake_path, commit_id, attr_path),
        Source::File(file_path) => get_drv_path_file(repo_root, file_path, commit_id, attr_path),
    }
}

fn get_drv_path_flake(
    flake_path: &FlakePath,
    commit_id: Option<&str>,
    attr_path: &AttrPath,
) -> eyre::Result<String> {
    let fragment = attr_path.to_flake_fragment().wrap_err_with(|| {
        format!("failed to evaluate derivation path of flake output {attr_path:?}")
    })?;
    let flake_ref = if let Some(rev) = commit_id {
        format!("{}?rev={}#{}", flake_path.as_str(), rev, fragment)
    } else {
        format!("{}#{}", flake_path.as_str(), fragment)
    };

    Cmd::nix()
        .args(["--extra-experimental-features", "nix-command flakes"])
        .args(["eval", "--json"])
        // Eval cache hardly works, sometimes it even seems to make things slower.
        // It also causes Nix to report "SQLite database is busy" errors
        // when running multiple evaluations in parallel.
        .arg("--no-eval-cache")
        .args(["--apply", "v: v.drvPath"])
        .args(["--", &flake_ref])
        .output_json()
}

fn get_drv_path_file(
    repo_root: &Path,
    file_path: &Path,
    commit_id: Option<&str>,
    attr_path: &AttrPath,
) -> eyre::Result<String> {
    let Ok(path_relative) = file_path.strip_prefix(repo_root) else {
        unreachable!("repo_root must be a prefix of file_path")
    };
    let attr_path_json = attr_path.to_parts_json();

    Cmd::nix_instantiate()
        .args(["--eval", "--strict", "--json", "--read-write-mode"])
        .args(["--expr", include_str!("nix/get-drv-path-file.nix")])
        .args(["--argstr", "repoRoot"])
        .arg(repo_root)
        .args(["--argstr", "pathInRepo"])
        .arg(path_relative)
        .args(if let Some(rev) = commit_id {
            ["--argstr", "rev", rev]
        } else {
            ["--arg", "rev", "null"]
        })
        .args(["--argstr", "attrPathJson", &attr_path_json])
        .output_json()
}
