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

pub(crate) fn working_tree_is_clean(repo_root: &Path) -> eyre::Result<bool> {
    let exit_code = Cmd::git()
        .arg("-C")
        .arg(repo_root)
        .args(["--git-dir", ".git"])
        .args(["diff", "--quiet", "HEAD"])
        .run_for_exit_code(0..=1)?;
    Ok(exit_code == 0)
}

pub(crate) fn resolve_commit(commit: &str, repo_root: &Path) -> eyre::Result<String> {
    let mut output = Cmd::git()
        .arg("-C")
        .arg(repo_root)
        .args(["--git-dir", ".git"])
        .args(["rev-parse", "--verify", "--end-of-options", commit])
        .output_string()?;
    strip_trailing_newline(&mut output)?;

    if output.len() != 40 {
        bail!("expected 'git rev-parse' command output to be 40 characters long, got {output:?}");
    }
    if output.chars().any(|c| !c.is_ascii_hexdigit()) {
        bail!(
            "expected 'git rev-parse' command output to contain only hexadecimal digits, \
            got {output:?}"
        );
    }

    Ok(output)
}

pub(crate) fn show_commit(commit_id: &str, repo_root: &Path) -> eyre::Result<String> {
    let mut output = Cmd::git()
        .arg("-C")
        .arg(repo_root)
        .args(["--git-dir", ".git"])
        .args([
            "show",
            "--color=always",
            "--pretty=oneline",
            "--abbrev-commit",
            "--decorate=short",
            "--no-patch",
            "--end-of-options",
            commit_id,
        ])
        .output_string()?;
    strip_trailing_newline(&mut output)?;

    Ok(output)
}

fn strip_trailing_newline(git_output: &mut String) -> eyre::Result<()> {
    match git_output.pop() {
        Some('\n') => Ok(()),
        _ => bail!("expected 'git' command output to end with a newline, got {git_output:?}"),
    }
}
