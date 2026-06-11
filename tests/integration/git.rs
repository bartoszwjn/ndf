use std::{path::Path, process::Command};

use crate::command;

pub(crate) struct GitRev {
    pub(crate) id: String,
    pub(crate) short_id: String,
}

pub(crate) fn init(dir: &Path) -> eyre::Result<()> {
    command::run(git_at(dir).args(["init", "--initial-branch", "main"]))?;
    Ok(())
}

pub(crate) fn switch(dir: &Path, to: &str) -> eyre::Result<()> {
    command::run(git_at(dir).args(["switch", "--detach", to]))?;
    Ok(())
}

pub(crate) fn commit(dir: &Path, message: &str) -> eyre::Result<GitRev> {
    add(dir)?;
    command::run(git_at(dir).args(["commit", "--allow-empty", "--message", message]))?;
    let id = rev_parse(dir, ["--verify", "@"])?;
    keep(dir, &id)?;
    Ok(GitRev {
        id,
        short_id: rev_parse(dir, ["--short", "@"])?,
    })
}

pub(crate) fn merge<'a>(
    dir: &Path,
    parents: impl IntoIterator<Item = &'a str>,
) -> eyre::Result<()> {
    let mut parents = parents.into_iter();
    switch(dir, parents.next().unwrap())?;
    command::run_allow_exit_codes(
        git_at(dir)
            .args(["merge", "--no-ff", "--no-commit"])
            .args(parents),
        0..=1,
    )?;
    Ok(())
}

pub(crate) fn add(dir: &Path) -> eyre::Result<()> {
    command::run(git_at(dir).args(["add", "--all"]))?;
    Ok(())
}

fn rev_parse<'a>(dir: &Path, args: impl IntoIterator<Item = &'a str>) -> eyre::Result<String> {
    let output = command::run(git_at(dir).arg("rev-parse").args(args))?;
    let mut rev = String::from_utf8(output.stdout)?;
    while rev.ends_with("\n") {
        rev.pop();
    }
    Ok(rev)
}

fn keep(dir: &Path, commit_id: &str) -> eyre::Result<()> {
    command::run(git_at(dir).args(["update-ref", &format!("refs/keep/{commit_id}"), commit_id]))?;
    Ok(())
}

fn git_at(dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir);
    cmd.args([
        "-c",
        "user.name=nobody",
        "-c",
        "user.email=nobody@example.tld",
    ]);
    cmd
}
