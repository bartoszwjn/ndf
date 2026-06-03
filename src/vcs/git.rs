use std::path::Path;

use eyre::bail;

use crate::command::Cmd;

pub(super) fn working_tree_is_clean(worktree_root: &Path) -> eyre::Result<bool> {
    let exit_code = Cmd::git()
        .arg("-C")
        .arg(worktree_root)
        .args(["--git-dir", ".git"])
        .args(["diff", "--quiet", "HEAD"])
        .run_for_exit_code(0..=1)?;

    Ok(exit_code == 0)
}

pub(super) fn resolve_commit(commit: &str, worktree_root: &Path) -> eyre::Result<String> {
    let mut output = Cmd::git()
        .arg("-C")
        .arg(worktree_root)
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

pub(super) fn show_commit(commit_id: &str, worktree_root: &Path) -> eyre::Result<String> {
    let mut output = Cmd::git()
        .arg("-C")
        .arg(worktree_root)
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
    match git_output.strip_suffix('\n') {
        Some(rest) => {
            git_output.truncate(rest.len());
            Ok(())
        }
        None => bail!("expected 'git' command output to end with a newline, got {git_output:?}"),
    }
}
