use std::{fmt, path::Path, sync::Mutex};

use crate::{
    attr_path::AttrPath,
    command::Cmd,
    source::{FlakePath, Source},
};

#[cfg(test)]
mod tests;

fn get_current_system() -> eyre::Result<String> {
    static CURRENT_SYSTEM: Mutex<Option<String>> = Mutex::new(None);

    let mut lock = CURRENT_SYSTEM.lock().unwrap();
    match &*lock {
        Some(system) => Ok(system.clone()),
        None => {
            let system: String = Cmd::nix_instantiate()
                .args(["--eval", "--strict", "--json"])
                .args(["--expr", "builtins.currentSystem"])
                .output_json()?;
            *lock = Some(system.clone());
            Ok(system)
        }
    }
}

pub(crate) fn get_flake_packages(
    flake_path: &FlakePath,
    commit_id: Option<&str>,
    impure: bool,
) -> eyre::Result<Vec<String>> {
    let current_system = get_current_system()?;
    let system = to_string_literal(&current_system);
    Cmd::nix()
        .args(["--extra-experimental-features", "nix-command flakes"])
        .args(["eval", "--json"])
        .args(if impure { ["--impure"].as_slice() } else { &[] })
        .args([
            "--apply",
            &format!("flake: builtins.attrNames (flake.packages.{system} or {{ }})"),
        ])
        .args(["--", &make_flake_root_output(flake_path, commit_id)])
        .output_json()
}

pub(crate) fn get_flake_nixos_configurations(
    flake_path: &FlakePath,
    commit_id: Option<&str>,
    impure: bool,
) -> eyre::Result<Vec<String>> {
    Cmd::nix()
        .args(["--extra-experimental-features", "nix-command flakes"])
        .args(["eval", "--json"])
        .args(if impure { ["--impure"].as_slice() } else { &[] })
        .args([
            "--apply",
            "flake: builtins.attrNames (flake.nixosConfigurations or {})",
        ])
        .args(["--", &make_flake_root_output(flake_path, commit_id)])
        .output_json()
}

pub(crate) fn get_file_output_attributes(
    repo_root: &Path,
    file_path: &Path,
    commit_id: Option<&str>,
) -> eyre::Result<Vec<String>> {
    Cmd::nix_instantiate()
        .args(["--eval", "--strict", "--json"])
        .args(["--expr", include_str!("nix/get-file-output-attributes.nix")])
        .args(["--argstr", "repoRoot"])
        .arg(repo_root)
        .args(["--argstr", "pathInRepo"])
        .arg(make_path_in_repo(repo_root, file_path))
        .args(if let Some(rev) = commit_id {
            ["--argstr", "rev", rev]
        } else {
            ["--arg", "rev", "null"]
        })
        .output_json()
}

pub(crate) fn prefetch_flake(flake_path: &FlakePath, commit_id: Option<&str>) -> eyre::Result<()> {
    Cmd::nix()
        .args(["--extra-experimental-features", "nix-command flakes"])
        .args(["flake", "archive"])
        .args(["--", &make_flake_ref(flake_path, commit_id).to_string()])
        .run_capture_stdio()
}

pub(crate) fn prefetch_repo(repo_root: &Path, commit_id: &str) -> eyre::Result<()> {
    Cmd::nix_instantiate()
        .args(["--eval", "--strict", "--json"])
        .args(["--expr", include_str!("nix/fetch-repo.nix")])
        .args(["--argstr", "repoRoot"])
        .arg(repo_root)
        .args(["--argstr", "rev", commit_id])
        .run_capture_stdio()
}

pub(crate) fn get_drv_path(
    repo_root: &Path,
    source: &Source,
    commit_id: Option<&str>,
    attr_path: &AttrPath,
    impure: bool,
) -> eyre::Result<Option<String>> {
    let result = match source {
        Source::Flake(flake_path) => get_drv_path_flake(flake_path, commit_id, attr_path, impure)?,
        Source::File(file_path) => get_drv_path_file(repo_root, file_path, commit_id, attr_path)?,
    };
    match result {
        GetDrvPathResult::Ok(drv_path) => Ok(Some(drv_path)),
        GetDrvPathResult::Missing => Ok(None),
        GetDrvPathResult::UnexpectedType(actual_type) => {
            Err(eyre::eyre!("expected a derivation, got {actual_type}"))
        }
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
enum GetDrvPathResult {
    Ok(String),
    Missing,
    UnexpectedType(String),
}

fn get_drv_path_flake(
    flake_path: &FlakePath,
    commit_id: Option<&str>,
    attr_path: &AttrPath,
    impure: bool,
) -> eyre::Result<GetDrvPathResult> {
    let apply_expr_base = include_str!("nix/get-drv-path-flake.nix");
    let (has_leading_dot, attr_path_parts) = attr_path.flake_query();
    let attr_path_expr = to_string_list_literal(&attr_path_parts);
    let apply_expr = if has_leading_dot {
        format!("({apply_expr_base}) {{ attrPath = {attr_path_expr}; system = null; }}")
    } else {
        let current_system = get_current_system()?;
        let system_expr = to_string_literal(&current_system);
        format!("({apply_expr_base}) {{ attrPath = {attr_path_expr}; system = {system_expr}; }}")
    };

    Cmd::nix()
        .args(["--extra-experimental-features", "nix-command flakes"])
        .args(["eval", "--json"])
        .args(if impure { ["--impure"].as_slice() } else { &[] })
        // Eval cache hardly works, sometimes it even seems to make things slower.
        // It also causes Nix to report "SQLite database is busy" errors
        // when running multiple evaluations in parallel.
        .arg("--no-eval-cache")
        .args(["--apply", &apply_expr])
        .args(["--", &make_flake_root_output(flake_path, commit_id)])
        .output_json()
}

fn get_drv_path_file(
    repo_root: &Path,
    file_path: &Path,
    commit_id: Option<&str>,
    attr_path: &AttrPath,
) -> eyre::Result<GetDrvPathResult> {
    let attr_path_json = serde_json::to_string::<Vec<&str>>(&attr_path.file_query())
        .expect("serializing a list of strings into a String cannot fail");

    Cmd::nix_instantiate()
        .args(["--eval", "--strict", "--json", "--read-write-mode"])
        .args(["--expr", include_str!("nix/get-drv-path-file.nix")])
        .args(["--argstr", "repoRoot"])
        .arg(repo_root)
        .args(["--argstr", "pathInRepo"])
        .arg(make_path_in_repo(repo_root, file_path))
        .args(if let Some(rev) = commit_id {
            ["--argstr", "rev", rev]
        } else {
            ["--arg", "rev", "null"]
        })
        .args(["--argstr", "attrPathJson", &attr_path_json])
        .output_json()
}

fn make_flake_ref(flake_path: &FlakePath, commit_id: Option<&str>) -> impl fmt::Display {
    fmt::from_fn(move |f| {
        let path = flake_path.as_str();
        match commit_id {
            Some(rev) => write!(f, "{path}?rev={rev}"),
            None => write!(f, "{path}"),
        }
    })
}

fn make_flake_root_output(flake_path: &FlakePath, commit_id: Option<&str>) -> String {
    format!("{}#.", make_flake_ref(flake_path, commit_id))
}

fn make_path_in_repo<'a>(repo_root: &Path, file_path: &'a Path) -> &'a Path {
    file_path
        .strip_prefix(repo_root)
        .expect("repo_root must be a prefix of file_path")
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

fn to_string_list_literal(elems: &[&str]) -> impl fmt::Display {
    use std::fmt::Write;

    fmt::from_fn(move |f| {
        f.write_char('[')?;
        let mut first = true;
        for elem in elems {
            if first {
                first = false;
            } else {
                f.write_char(' ')?;
            }
            write!(f, "{}", to_string_literal(elem))?;
        }
        f.write_char(']')?;
        Ok(())
    })
}
