use std::path::Path;

use eyre::{WrapErr, bail};

use crate::{
    command::Cmd,
    diff_spec::{AttrPath, GitRev, Source},
    git,
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

pub(crate) fn get_current_flake_packages() -> eyre::Result<Vec<String>> {
    let current_system = get_current_system()?;
    let package_names_fn =
        format!("flake: builtins.attrNames (flake.packages.{current_system} or {{}})");
    Cmd::nix()
        .args(["eval", "--json", ".#.", "--apply", &package_names_fn])
        .output_json()
}

pub(crate) fn get_current_flake_nixos_configurations() -> eyre::Result<Vec<String>> {
    let nixos_names_fn = "flake: builtins.attrNames (flake.nixosConfigurations or {})";
    Cmd::nix()
        .args(["eval", "--json", ".#.", "--apply", nixos_names_fn])
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
    source: &Source,
    git_rev: &GitRev,
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
        Source::FlakeCurrentDir => {
            let flake_ref = match git_rev {
                GitRev::Worktree => String::from(".#"),
                GitRev::Rev { rev, .. } => format!(".?rev={rev}#"),
            } + &attr_path.0;
            cmd.args(["--", &flake_ref]).output_json()
        }
        Source::File(path) => match git_rev {
            GitRev::Worktree => cmd
                .arg("--file")
                .arg(path)
                .args(["--", &attr_path.0])
                .output_json(),
            GitRev::Rev { rev, .. } => {
                let repo_root = git::get_repo_root(path)?;
                let path_absolute = path
                    .canonicalize()
                    .wrap_err_with(|| format!("failed to resolve {:?}", path))?;
                let Ok(path_from_repo_root) = path_absolute.strip_prefix(&repo_root) else {
                    bail!(
                        "Path to repository root reported by 'git' \
                        is not a prefix of the given file path {path_absolute:?}"
                    );
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
                    .arg(&repo_root)
                    .args(["--argstr", "pathInRepo"])
                    .arg(path_from_repo_root)
                    .args(["--argstr", "rev", rev])
                    .args(["--", &attr_path.0])
                    .output_json()
            }
        },
    }
}
