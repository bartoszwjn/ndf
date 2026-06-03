use std::{
    fmt,
    path::{Path, PathBuf},
};

use color_eyre::Section;
use eyre::{WrapErr, bail, eyre};

use crate::source::Source;

mod git;
mod jj;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VcsMode {
    Git,
    Jujutsu,
}

#[derive(Debug)]
pub(crate) struct Repository {
    /// Absolute, canonicalized path to repository root.
    root: PathBuf,
    mode: VcsMode,
    working_tree_is_clean: Option<bool>,
}

impl Repository {
    pub(crate) fn for_source(
        source: &Source,
        mode_override: Option<VcsMode>,
    ) -> eyre::Result<Self> {
        let path_in_repo = match source {
            Source::Flake(flake_path) => flake_path.as_path(),
            Source::File(file_path) => file_path,
        };
        let (root, mode) = locate_repo(path_in_repo, mode_override)?;
        if let VcsMode::Jujutsu = mode {
            jj::git_import(&root)?;
        }
        Ok(Self {
            root,
            mode,
            working_tree_is_clean: None,
        })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn mode(&self) -> VcsMode {
        self.mode
    }

    pub(crate) fn working_tree_is_clean(&mut self) -> eyre::Result<bool> {
        match self.working_tree_is_clean {
            Some(cached) => Ok(cached),
            None => {
                let is_clean = match self.mode {
                    VcsMode::Git => git::working_tree_is_clean(self.root())?,
                    VcsMode::Jujutsu => jj::working_copy_commit_is_empty(self.root())?,
                };
                Ok(*self.working_tree_is_clean.insert(is_clean))
            }
        }
    }

    pub(crate) fn resolve_commit(&self, commit: &str) -> eyre::Result<Revision> {
        match self.mode {
            // NOTE: `[working tree]` should never be a valid input to `git rev-parse`,
            // a.k.a. "extended SHA-1 syntax":
            // https://git-scm.com/docs/git-rev-parse#_specifying_revisions
            //
            // Note that ref names cannot contain '[':
            // https://git-scm.com/docs/git-check-ref-format
            VcsMode::Git if commit == "[working tree]" => Ok(Revision::GitWorkingTree),
            VcsMode::Git => {
                let commit_id = git::resolve_commit(commit, self.root())?;
                let display = git::show_commit(&commit_id, self.root())?;
                Ok(Revision::Commit(Commit { commit_id, display }))
            }
            // Special case for a clearer error message.
            VcsMode::Jujutsu if commit == "[working tree]" => Err(eyre!(
                "{commit:?} is not a valid Jujutsu revset"
            )
            .note(
                "The Jujutsu equivalent of Git's working tree is the working copy commit (`@`). \
                Use `--git` to switch to Git mode.",
            )),
            VcsMode::Jujutsu => {
                let (commit_id, display) = jj::resolve_and_show_commit(commit, self.root())?;
                Ok(Revision::Commit(Commit { commit_id, display }))
            }
        }
    }
}

impl fmt::Display for Repository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.root.display())
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Revision {
    Commit(Commit),
    GitWorkingTree,
}

#[derive(Clone, Debug)]
pub(crate) struct Commit {
    commit_id: String,
    display: String,
}

impl Revision {
    pub(crate) fn commit_id(&self) -> Option<&str> {
        match self {
            Revision::Commit(commit) => Some(&commit.commit_id),
            Revision::GitWorkingTree => None,
        }
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Commit(commit) => f.write_str(&commit.display),
            Self::GitWorkingTree => {
                use crate::styles::WORKING_TREE;
                write!(f, "{WORKING_TREE}[working tree]{WORKING_TREE:#}")
            }
        }
    }
}

fn locate_repo(
    path_in_repo: &Path,
    mode_override: Option<VcsMode>,
) -> eyre::Result<(PathBuf, VcsMode)> {
    assert!(path_in_repo.is_absolute());

    let mut path = path_in_repo.to_owned();
    let metadata = path
        .metadata()
        .wrap_err_with(|| format!("failed to query metadata of {path:?}"))?;
    if metadata.is_file() {
        path.pop();
    }

    loop {
        let has_dot_git = marker_exists(&mut path, ".git")?;
        let has_dot_jj = match mode_override {
            Some(VcsMode::Git) => false,
            Some(VcsMode::Jujutsu) | None => marker_exists(&mut path, ".jj")?,
        };

        match (has_dot_git, has_dot_jj, mode_override) {
            (true, true, Some(VcsMode::Jujutsu) | None) => return Ok((path, VcsMode::Jujutsu)),
            (true, false, Some(VcsMode::Git) | None) => return Ok((path, VcsMode::Git)),

            (true, false, Some(VcsMode::Jujutsu)) => {
                return Err(eyre!(
                    "Git workspace at {path:?} is not a Jujutsu workspace"
                ));
            }
            (false, true, Some(VcsMode::Jujutsu) | None) => {
                return Err(
                    eyre!("Jujutsu workspace at {path:?} is not a Git workspace")
                        .note("Jujutsu workspaces without Git colocation are not supported"),
                );
            }

            (_, true, Some(VcsMode::Git)) => unreachable!(),
            (false, false, _) => {}
        }

        if &path == "/" {
            let desc = match mode_override {
                Some(VcsMode::Git) => "Git",
                Some(VcsMode::Jujutsu) => "Jujutsu",
                None => "Git or Jujutsu",
            };
            bail!("path {path_in_repo:?} is not part of a {desc} workspace");
        }

        assert!(path.pop());
    }
}

fn marker_exists(path: &mut PathBuf, marker_name: &str) -> eyre::Result<bool> {
    path.push(marker_name);
    let res = path
        .try_exists()
        .wrap_err_with(|| format!("failed to check for existence of {path:?}"));
    path.pop();
    res
}
