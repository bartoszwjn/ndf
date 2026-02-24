use std::{
    ffi::OsString,
    os::unix::ffi::OsStringExt,
    path::{Path, PathBuf},
};

use eyre::{WrapErr, bail};

use crate::command::Cmd;

pub(crate) fn resolve_ref(git_ref: &str, path_in_repo: &Path) -> eyre::Result<String> {
    let dir_in_repo = get_dir_in_repo(path_in_repo)?;

    let mut output = Cmd::git()
        .args(["rev-parse", "--verify", "--end-of-options", git_ref])
        .current_dir(dir_in_repo)
        .output()?;
    strip_trailing_newline(&mut output)?;
    let output =
        String::from_utf8(output).wrap_err("output of 'git rev-parse' is not valid utf8")?;

    assert_eq!(
        output.len(),
        40,
        "'git rev-parse' output length is not 40: {}",
        output.len()
    );
    assert!(
        output.chars().all(|c| c.is_ascii_hexdigit()),
        "'git rev-parse' output contains unexpected characters: {output:?}"
    );

    Ok(output)
}

pub(crate) fn get_repo_root(path_in_repo: &Path) -> eyre::Result<PathBuf> {
    let dir_in_repo = get_dir_in_repo(path_in_repo)?;

    let mut output = Cmd::git()
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir_in_repo)
        .output()?;
    strip_trailing_newline(&mut output)?;

    Ok(PathBuf::from(OsString::from_vec(output)))
}

fn get_dir_in_repo(path_in_repo: &Path) -> eyre::Result<&Path> {
    let path_metadata = path_in_repo
        .metadata()
        .wrap_err_with(|| format!("failed to query metadata of {:?}", path_in_repo))?;

    if path_metadata.is_dir() {
        Ok(path_in_repo)
    } else {
        match path_in_repo.parent() {
            Some(parent) if parent.as_os_str().is_empty() => Ok(Path::new(".")),
            Some(parent) => Ok(parent),
            None => unreachable!("path points to a non-directory and doesn't have a parent"),
        }
    }
}

fn strip_trailing_newline(git_output: &mut Vec<u8>) -> eyre::Result<()> {
    match git_output.last() {
        Some(b'\n') => {
            git_output.pop();
            Ok(())
        }
        _ => bail!(
            "expected a newline character at the end of 'git' command output, got {:?}",
            String::from_utf8_lossy(git_output)
        ),
    }
}
