use std::{path::Path, process::Command};

use crate::command;

pub(crate) struct JjRev {
    pub(crate) commit_id: String,
    pub(crate) commit_short_id: String,
    pub(crate) log_oneline: String,
}

pub(crate) fn init(dir: &Path, colocate: bool) -> eyre::Result<()> {
    command::run(jj_at(dir).args([
        "git",
        "init",
        if colocate {
            "--colocate"
        } else {
            "--no-colocate"
        },
    ]))?;
    Ok(())
}

pub(crate) fn new<'a>(
    dir: &Path,
    message: &str,
    parents: impl IntoIterator<Item = &'a str>,
) -> eyre::Result<()> {
    command::run(jj_at(dir).args(["new", "--message", message]).args(parents))?;
    Ok(())
}

pub(crate) fn get_rev(dir: &Path) -> eyre::Result<JjRev> {
    let output = command::run(jj_at(dir).args([
        "log",
        "--no-graph",
        "--template",
        "join(';', commit_id, commit_id.shortest(7), builtin_log_oneline)",
        "--revision",
        "@",
    ]))?;

    let s = str::from_utf8(&output.stdout).unwrap();
    let (commit_id, s) = s.split_once(';').unwrap();
    let (commit_short_id, s) = s.split_once(';').unwrap();
    let log_oneline = s.strip_suffix('\n').unwrap();
    Ok(JjRev {
        commit_id: commit_id.to_owned(),
        commit_short_id: commit_short_id.to_owned(),
        log_oneline: log_oneline.to_owned(),
    })
}

fn jj_at(dir: &Path) -> Command {
    let mut cmd = Command::new("jj");
    cmd.current_dir(dir);
    cmd.args([
        "--config",
        "user={ name = 'nobody', email = 'nobody@example.tld' }",
    ]);
    cmd
}
