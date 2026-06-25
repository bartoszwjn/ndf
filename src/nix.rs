use std::{ffi::OsStr, fmt, path::Path, sync::Mutex};

use crate::{
    attr_path::AttrPath,
    command::Cmd,
    glob::{AttrQuery, FlakeAttrPathQuery, Pattern},
    source::{FlakePath, Source},
};

#[cfg(test)]
mod tests;

fn nix_instantiate_eval_json() -> Cmd {
    let mut cmd = Cmd::nix_instantiate();
    cmd.args(["--eval", "--strict", "--json"]);
    cmd
}

fn nix_eval_json(impure: bool) -> Cmd {
    let mut cmd = Cmd::nix();
    cmd.args(["--extra-experimental-features", "nix-command flakes"]);
    cmd.args(["eval", "--json"]);
    if impure {
        cmd.arg("--impure");
    }
    cmd
}

impl Cmd {
    fn nix_argstr(&mut self, name: &str, value: impl AsRef<OsStr>) -> &mut Cmd {
        self.args(["--argstr", name]);
        self.arg(value);
        self
    }

    fn nix_argstr_nullable(&mut self, name: &str, value: Option<impl AsRef<OsStr>>) -> &mut Cmd {
        match value {
            Some(value) => {
                self.args(["--argstr", name]);
                self.arg(value);
            }
            None => {
                self.args(["--arg", name, "null"]);
            }
        }
        self
    }
}

fn get_current_system() -> eyre::Result<String> {
    static CURRENT_SYSTEM: Mutex<Option<String>> = Mutex::new(None);

    let mut lock = CURRENT_SYSTEM.lock().unwrap();
    match &*lock {
        Some(system) => Ok(system.clone()),
        None => {
            let system: String = nix_instantiate_eval_json()
                .args(["--expr", "builtins.currentSystem"])
                .output_json()?;
            *lock = Some(system.clone());
            Ok(system)
        }
    }
}

pub(crate) fn get_default_flake_outputs(
    flake_path: &FlakePath,
    commit_id: Option<&str>,
    nixos: bool,
    impure: bool,
) -> eyre::Result<Vec<AttrPath>> {
    let apply_expr = if nixos {
        "flake: builtins.attrNames (flake.nixosConfigurations or { })"
    } else {
        let current_system = get_current_system()?;
        let system = to_string_literal(&current_system);
        &format!("flake: builtins.attrNames (flake.packages.{system} or {{ }})")
    };

    let output = nix_eval_json(impure)
        .args(["--apply", apply_expr])
        .args(["--", &make_flake_root_output(flake_path, commit_id)])
        .output_json::<Vec<String>>()?;

    Ok(map_vec(output, |name| {
        AttrPath::new(false, vec![name], nixos)
    }))
}

pub(crate) fn get_default_file_outputs(
    repo_root: &Path,
    file_path: &Path,
    commit_id: Option<&str>,
    nixos: bool,
) -> eyre::Result<Vec<AttrPath>> {
    let output = nix_instantiate_eval_json()
        .args(["--expr", include_str!("nix/get-file-output-attributes.nix")])
        .nix_argstr("repoRoot", repo_root)
        .nix_argstr("pathInRepo", make_path_in_repo(repo_root, file_path))
        .nix_argstr_nullable("rev", commit_id)
        .output_json::<Vec<String>>()?;

    Ok(map_vec(output, |name| {
        AttrPath::new(false, vec![name], nixos)
    }))
}

pub(crate) fn get_matching_flake_outputs(
    flake_path: &FlakePath,
    commit_id: Option<&str>,
    nixos: bool,
    impure: bool,
    patterns: &[Pattern],
) -> eyre::Result<Vec<Vec<AttrPath>>> {
    let queries = patterns.iter().map(|pat| pat.flake_query()).collect();
    let queries_json = serde_json::to_string::<Vec<FlakeAttrPathQuery>>(&queries)
        .expect("serializing FlakeAttrPathQuery should never fail");
    let queries_json_expr = to_string_literal(&queries_json);

    let current_system = get_current_system()?;
    let system_expr = to_string_literal(&current_system);

    let apply_base_expr = include_str!("nix/get-matching-flake-outputs.nix");
    let apply_expr = format!(
        "({apply_base_expr}) {{ \
            queriesJson = {queries_json_expr}; \
            nixos = {nixos}; \
            system = {system_expr}; \
        }}"
    );

    let output = nix_eval_json(impure)
        .args(["--apply", &apply_expr])
        .args(["--", &make_flake_root_output(flake_path, commit_id)])
        .output_json::<Vec<Vec<Vec<String>>>>()?;

    if queries.len() != output.len() {
        eyre::bail!(
            "nix eval result contains an unexpected number of elements \
            (expected {}, got {})",
            queries.len(),
            output.len(),
        );
    }

    Ok(output
        .into_iter()
        .zip(&queries)
        .map(|(query_results, query)| {
            map_vec(query_results, |result| {
                AttrPath::new(query.leading_dot, result, nixos)
            })
        })
        .collect())
}

pub(crate) fn get_matching_file_outputs(
    repo_root: &Path,
    file_path: &Path,
    commit_id: Option<&str>,
    nixos: bool,
    patterns: &[Pattern],
) -> eyre::Result<Vec<Vec<AttrPath>>> {
    let queries = patterns.iter().map(|pat| pat.file_query()).collect();
    let queries_json = serde_json::to_string::<Vec<Vec<AttrQuery>>>(&queries)
        .expect("serializing AttrQuery should never fail");

    let output = nix_instantiate_eval_json()
        .args(["--expr", include_str!("nix/get-matching-file-outputs.nix")])
        .nix_argstr("repoRoot", repo_root)
        .nix_argstr("pathInRepo", make_path_in_repo(repo_root, file_path))
        .nix_argstr_nullable("rev", commit_id)
        .nix_argstr("queriesJson", &queries_json)
        .output_json::<Vec<Vec<Vec<String>>>>()?;

    if queries.len() != output.len() {
        eyre::bail!(
            "nix-instantiate result contains an unexpected number of elements \
            (expected {}, got {})",
            queries.len(),
            output.len(),
        );
    }

    Ok(map_vec(output, |query_results| {
        map_vec(query_results, |result| AttrPath::new(false, result, nixos))
    }))
}

pub(crate) fn prefetch_flake(flake_path: &FlakePath, commit_id: Option<&str>) -> eyre::Result<()> {
    Cmd::nix()
        .args(["--extra-experimental-features", "nix-command flakes"])
        .args(["flake", "archive"])
        .args(["--", &make_flake_ref(flake_path, commit_id).to_string()])
        .run_capture_stdio()
}

pub(crate) fn prefetch_repo(repo_root: &Path, commit_id: &str) -> eyre::Result<()> {
    nix_instantiate_eval_json()
        .args(["--expr", include_str!("nix/fetch-repo.nix")])
        .nix_argstr("repoRoot", repo_root)
        .nix_argstr("rev", commit_id)
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

    nix_eval_json(impure)
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

    nix_instantiate_eval_json()
        .arg("--read-write-mode")
        .args(["--expr", include_str!("nix/get-drv-path-file.nix")])
        .nix_argstr("repoRoot", repo_root)
        .nix_argstr("pathInRepo", make_path_in_repo(repo_root, file_path))
        .nix_argstr_nullable("rev", commit_id)
        .nix_argstr("attrPathJson", &attr_path_json)
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
    fmt::from_fn(move |f| {
        write!(f, "[")?;
        let mut elems = elems.iter().peekable();
        while let Some(elem) = elems.next() {
            write!(f, "{}", to_string_literal(elem))?;
            if elems.peek().is_some() {
                write!(f, " ")?;
            }
        }
        write!(f, "]")?;
        Ok(())
    })
}

fn map_vec<T, U>(vec: Vec<T>, f: impl FnMut(T) -> U) -> Vec<U> {
    vec.into_iter().map(f).collect()
}
