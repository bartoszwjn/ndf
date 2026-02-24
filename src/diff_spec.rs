use std::path::PathBuf;

use crate::{
    cli::DiffTool,
    color::{BLUE, BOLD, CYAN, GREEN, MAGENTA, YELLOW},
};

#[derive(Clone, Debug)]
pub(crate) struct DiffSpec {
    pub(crate) source: Source,
    pub(crate) from: GitRev,
    pub(crate) to: GitRev,
    pub(crate) tool: DiffTool,
    pub(crate) base: Option<AttrPath>,
    pub(crate) attr_paths: Vec<AttrPath>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Source {
    FlakeCurrentDir,
    File(PathBuf),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum GitRev {
    Rev { orig_ref: String, rev: String },
    Worktree,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct AttrPath(pub(crate) String);

impl std::fmt::Display for DiffSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        macro_rules! header {
            ($name:expr) => {
                format_args!("{BOLD}{: <6}{BOLD:#}", format!("{}:", $name))
            };
        }

        match &self.source {
            Source::FlakeCurrentDir => writeln!(f, "{} {BLUE}.{BLUE:#}", header!("Flake"))?,
            Source::File(path) => {
                writeln!(f, "{} {BLUE}{}{BLUE:#}", header!("File"), path.display())?
            }
        }

        writeln!(f, "{} {}", header!("From"), self.from)?;
        writeln!(f, "{} {}", header!("To"), self.to)?;

        let tool = match self.tool {
            DiffTool::None => "none",
            DiffTool::NixDiff => "nix-diff",
        };
        writeln!(f, "{} {}", header!("Tool"), tool)?;

        if let Some(base) = &self.base {
            writeln!(f, "{} {}", header!("Base"), base)?;
        }
        writeln!(f, "{}", header!("Attribute paths"))?;
        for attr_path in &self.attr_paths {
            writeln!(f, "  {attr_path}")?;
        }

        Ok(())
    }
}

impl std::fmt::Display for GitRev {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rev { orig_ref, rev } => {
                write!(f, "{GREEN}{orig_ref}{GREEN:#} {YELLOW}{rev}{YELLOW:#}")
            }
            // NOTE: ref names cannot contain '[', see `git check-ref-format --help`.
            Self::Worktree => {
                write!(f, "{MAGENTA}[worktree]{MAGENTA:#}")
            }
        }
    }
}

impl std::fmt::Display for AttrPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{CYAN}{}{CYAN:#}", self.0)
    }
}
