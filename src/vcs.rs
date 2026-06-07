use std::{
    fmt,
    path::{Path, PathBuf},
};

use crate::source::Source;

mod git;

#[derive(Debug)]
pub(crate) struct Repository {
    /// Absolute, canonicalized path to repository root.
    root: PathBuf,
    worktree_is_clean: Option<bool>,
}

impl Repository {
    pub(crate) fn for_source(source: &Source) -> eyre::Result<Self> {
        let root = git::get_repo_root(match source {
            Source::Flake(flake_path) => flake_path.as_path(),
            Source::File(file_path) => file_path,
        })?;
        Ok(Self {
            root,
            worktree_is_clean: None,
        })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn worktree_is_clean(&mut self) -> eyre::Result<bool> {
        match self.worktree_is_clean {
            Some(cached) => Ok(cached),
            None => {
                let is_clean = git::working_tree_is_clean(self.root())?;
                Ok(*self.worktree_is_clean.insert(is_clean))
            }
        }
    }

    pub(crate) fn resolve_commit(&self, commit: Option<&str>) -> eyre::Result<Revision> {
        let Some(commit) = commit else {
            return Ok(Revision::GitWorktree);
        };

        let commit_id = git::resolve_commit(commit, self.root())?;
        let display = git::show_commit(&commit_id, self.root())?;
        Ok(Revision::GitRevision(GitRevision { commit_id, display }))
    }
}

impl fmt::Display for Repository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.root.display())
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Revision {
    GitRevision(GitRevision),
    GitWorktree,
}

#[derive(Clone, Debug)]
pub(crate) struct GitRevision {
    commit_id: String,
    display: String,
}

impl Revision {
    pub(crate) fn commit_id(&self) -> Option<&str> {
        match self {
            Revision::GitRevision(git_revision) => Some(&git_revision.commit_id),
            Revision::GitWorktree => None,
        }
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitRevision(git_revision) => f.write_str(&git_revision.display),
            // NOTE: ref names cannot contain '[', see `git check-ref-format --help`.
            Self::GitWorktree => {
                use crate::styles::WORKTREE;
                write!(f, "{WORKTREE}[worktree]{WORKTREE:#}")
            }
        }
    }
}
