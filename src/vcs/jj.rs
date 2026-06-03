use std::path::Path;

use eyre::bail;

use crate::command::Cmd;

pub(super) fn git_import(workspace_root: &Path) -> eyre::Result<()> {
    // NOTE: this is the only time we run `jj` without `--ignore-working-copy`
    Cmd::jj()
        .arg("--repository")
        .arg(workspace_root)
        .args(["git", "import"])
        .run_capture_stdio()
}

pub(super) fn working_copy_commit_is_empty(workspace_root: &Path) -> eyre::Result<bool> {
    Cmd::jj()
        .arg("--repository")
        .arg(workspace_root)
        .arg("--ignore-working-copy")
        .arg("log")
        .arg("--no-graph")
        .args(["--template", "if(empty, 'true', 'false')"])
        .args(["--revision", "exactly(@, 1)"])
        .output_json::<bool>()
}

pub(super) fn resolve_and_show_commit(
    commit: &str,
    workspace_root: &Path,
) -> eyre::Result<(String, String)> {
    let output = Cmd::jj()
        .arg("--repository")
        .arg(workspace_root)
        .arg("--ignore-working-copy")
        .arg("log")
        .arg("--no-graph")
        .args([
            "--template",
            "stringify(commit_id) ++ ';' ++ builtin_log_oneline",
        ])
        .args(["--color", "always"])
        .args(["--revision", &format!("exactly({commit}, 1)")])
        .output_string()?;

    let output = output.strip_suffix('\n').unwrap_or(&output);
    let Some((commit_id, display)) = output.split_once(';') else {
        bail!("could not parse the output of `jj log` (missing ';'): {output:?}");
    };

    if commit_id.len() != 40 {
        bail!("expected commit ID to be 40 characters long, got {commit_id:?}");
    }
    if commit_id.chars().any(|c| !c.is_ascii_hexdigit()) {
        bail!("expected commit ID to contain only hexadecimal digits, got {commit_id:?}");
    }

    Ok((commit_id.to_owned(), display.to_owned()))
}
