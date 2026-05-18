use std::{fmt, path::Path};

use eyre::WrapErr;

use crate::{
    attr_path::AttrPath,
    command::Cmd,
    diff_spec::{FlakePath, Source},
};

#[cfg(test)]
mod tests;

fn get_current_system() -> eyre::Result<String> {
    Cmd::nix_instantiate()
        .args(["--eval", "--strict", "--json"])
        .args(["--expr", "builtins.currentSystem"])
        .output_json()
}

pub(crate) fn get_flake_packages(flake_path: &FlakePath) -> eyre::Result<Vec<String>> {
    let current_system = get_current_system()?;
    let system = to_string_literal(&current_system);
    Cmd::nix()
        .args(["--extra-experimental-features", "nix-command flakes"])
        .args(["eval", "--json"])
        .args([
            "--apply",
            &format!("flake: builtins.attrNames (flake.packages.{system} or {{ }})"),
        ])
        .args(["--", &format!("{}#.", flake_path.as_str())])
        .output_json()
}

pub(crate) fn get_flake_nixos_configurations(flake_path: &FlakePath) -> eyre::Result<Vec<String>> {
    Cmd::nix()
        .args(["--extra-experimental-features", "nix-command flakes"])
        .args(["eval", "--json"])
        .args([
            "--apply",
            "flake: builtins.attrNames (flake.nixosConfigurations or {})",
        ])
        .args(["--", &format!("{}#.", flake_path.as_str())])
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
        .args(["--argstr", "attrPathJson", &attr_path.to_parts_json()])
        .output_json()
}

/// Formats a given string as a string literal value in the Nix expression language.
///
/// See: <https://nix.dev/manual/nix/2.34/language/string-literals.html>
fn to_string_literal(s: &str) -> impl fmt::Display {
    use std::fmt::Write;

    fmt::from_fn(move |f| {
        let mut s = s;
        f.write_char('"')?;
        while !s.is_empty() {
            let mut next_escape_ix = s
                .find(['"', '\\', '$', '\n', '\r', '\t'])
                .unwrap_or(s.len());
            while s[next_escape_ix..].starts_with('$') && !s[next_escape_ix..].starts_with("${") {
                next_escape_ix = s[next_escape_ix + 1..]
                    .find(['"', '\\', '$', '\n', '\r', '\t'])
                    .map(|ix| ix + next_escape_ix + 1)
                    .unwrap_or(s.len());
            }

            f.write_str(&s[..next_escape_ix])?;
            s = &s[next_escape_ix..];

            if let Some(c) = s.chars().next() {
                f.write_char('\\')?;
                match c {
                    '"' | '\\' | '$' => f.write_char(c)?,
                    '\n' => f.write_char('n')?,
                    '\r' => f.write_char('r')?,
                    '\t' => f.write_char('t')?,
                    _ => unreachable!(),
                }
                s = &s[1..];
            }
        }
        f.write_char('"')?;
        Ok(())
    })
}
