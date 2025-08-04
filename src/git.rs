use std::{path::Path, process::Command};

use anyhow::Context as _;

use crate::command;

pub(crate) fn resolve_ref(git_ref: &str, path_in_repo: &Path) -> anyhow::Result<String> {
    let path_metadata = path_in_repo
        .metadata()
        .with_context(|| format!("failed to query metadata of {}", path_in_repo.display()))?;
    let dir_in_repo = if path_metadata.is_dir() {
        path_in_repo
    } else {
        match path_in_repo.parent() {
            Some(parent) => parent,
            None => unreachable!("path points to a non-directory and doesn't have a parent"),
        }
    };

    let output = command::output(
        Command::new("git")
            .args(["rev-parse", "--verify", "--end-of-options", git_ref])
            .current_dir(dir_in_repo),
    )?;
    let mut output =
        String::from_utf8(output).context("output of 'git rev-parse' is not valid utf8")?;
    output.truncate(output.trim_end().len());

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
