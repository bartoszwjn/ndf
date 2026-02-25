use std::path::{Path, PathBuf};

use eyre::{WrapErr, bail};

use crate::command::Cmd;

pub(crate) fn get_repo_root(path_in_repo: &Path) -> eyre::Result<PathBuf> {
    assert!(path_in_repo.is_absolute());

    let mut path = path_in_repo.to_owned();
    let metadata = path
        .metadata()
        .wrap_err_with(|| format!("failed to query metadata of {path:?}"))?;
    if metadata.is_file() {
        path.pop();
    }
    loop {
        path.push(".git");
        let has_dot_git = path
            .try_exists()
            .wrap_err_with(|| format!("failed to check for existence of {path:?}"))?;
        path.pop();
        if has_dot_git {
            return Ok(path);
        }

        if &path == "/" {
            bail!("path {path_in_repo:?} is not part of a Git repository");
        }

        path.pop();
    }
}

pub(crate) fn resolve_commit(commit: &str, repo_root: &Path) -> eyre::Result<String> {
    let mut output = Cmd::git()
        .args(["rev-parse", "--verify", "--end-of-options", commit])
        .current_dir(repo_root)
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
