use std::{
    ffi::OsStr,
    path::Path,
    process::{Command, Output},
};

use color_eyre::{Section, SectionExt};

pub(crate) struct GitRev {
    pub(crate) id: String,
    pub(crate) short_id: String,
}

pub(crate) fn init(dir: &Path) -> eyre::Result<GitRev> {
    run(git_at(dir).args(["init", "--initial-branch", "main"]))?;
    commit(dir, "initial commit")
}

pub(crate) fn commit(dir: &Path, message: &str) -> eyre::Result<GitRev> {
    add(dir)?;
    run(git_at(dir)
        .args(["commit", "--allow-empty", "--message", message])
        .envs([
            ("GIT_AUTHOR_NAME", "nobody"),
            ("GIT_AUTHOR_EMAIL", "nobody@example.tld"),
            ("GIT_COMMITTER_NAME", "nobody"),
            ("GIT_COMMITTER_EMAIL", "nobody@example.tld"),
        ]))?;
    Ok(GitRev {
        id: rev_parse(dir, ["--verify", "@"])?,
        short_id: rev_parse(dir, ["--short", "@"])?,
    })
}

pub(crate) fn add(dir: &Path) -> eyre::Result<()> {
    run(git_at(dir).args(["add", "--all"]))?;
    Ok(())
}

fn rev_parse(
    dir: &Path,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> eyre::Result<String> {
    let output = run(git_at(dir).arg("rev-parse").args(args))?;
    let mut rev = String::from_utf8(output.stdout)?;
    while rev.ends_with("\n") {
        rev.pop();
    }
    Ok(rev)
}

fn git_at(dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir);
    cmd
}

fn run(cmd: &mut Command) -> eyre::Result<Output> {
    let output = cmd.output()?;
    if !output.status.success() {
        return Err(
            eyre::eyre!("git command failed ({})", output.status).section(
                String::from_utf8_lossy(&output.stderr)
                    .into_owned()
                    .header("Captured stderr:"),
            ),
        );
    }
    Ok(output)
}
