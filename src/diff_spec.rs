use std::{fmt, iter, path::PathBuf};

use crate::{attr_path::AttrPath, cli::DiffTool, display, source::Source};

#[derive(Clone, Debug)]
pub(crate) struct DiffSpec {
    pub(crate) source: Source,
    /// Absolute, canonicalized path to repository root.
    pub(crate) repo: PathBuf,
    pub(crate) from: Revision,
    pub(crate) to: Revision,
    pub(crate) impure: bool,
    pub(crate) tool: DiffTool,
    pub(crate) tool_extra_args: Vec<String>,
    pub(crate) base: Option<AttrPath>,
    pub(crate) attr_paths: Vec<AttrPath>,
}

#[derive(Clone, Debug)]
pub(crate) enum Revision {
    GitRevision { commit_id: String, display: String },
    GitWorktree,
}

impl Revision {
    pub(crate) fn commit_id(&self) -> Option<&str> {
        match self {
            Revision::GitRevision { commit_id, .. } => Some(commit_id),
            Revision::GitWorktree => None,
        }
    }
}

impl std::fmt::Display for DiffSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::styles::{HEADER, IMPURE, SOURCE};

        fn header(name: &str) -> impl fmt::Display {
            const MIN_WIDTH: usize = 5;
            let pad_width = MIN_WIDTH.saturating_sub(name.len());
            fmt::from_fn(move |f| write!(f, "{HEADER}{name}:{HEADER:#}{:pad_width$}", ""))
        }

        match &self.source {
            Source::Flake(flake_path) => {
                let flake_path = flake_path.as_str();
                write!(f, "{} {SOURCE}{flake_path}{SOURCE:#}", header("Flake"))?;
                if self.impure {
                    write!(f, " {IMPURE}(impure){IMPURE:#}")?;
                }
                writeln!(f)?;
            }
            Source::File(path) => {
                let path = path.display();
                writeln!(f, "{} {SOURCE}{path}{SOURCE:#}", header("File"))?
            }
        }

        writeln!(f, "{} {}", header("Repo"), self.repo.display())?;
        writeln!(f, "{} {}", header("From"), self.from)?;
        writeln!(f, "{} {}", header("To"), self.to)?;

        let tool = match self.tool {
            DiffTool::None => None,
            DiffTool::NixDiff => Some("nix-diff"),
        };
        if let Some(tool) = tool {
            let tool_cmd = display::display_command_args(|| {
                iter::chain([tool], self.tool_extra_args.iter().map(String::as_str))
            });
            writeln!(f, "{} {}", header("Tool"), tool_cmd)?;
        }

        if let Some(base) = &self.base {
            writeln!(f, "{} {}", header("Base"), base.display())?;
        }
        writeln!(f, "{}", header("Attribute paths"))?;
        for attr_path in &self.attr_paths {
            writeln!(f, "  {}", attr_path.display())?;
        }

        Ok(())
    }
}

impl std::fmt::Display for Revision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitRevision { display, .. } => f.write_str(display),
            // NOTE: ref names cannot contain '[', see `git check-ref-format --help`.
            Self::GitWorktree => {
                use crate::styles::WORKTREE;
                write!(f, "{WORKTREE}[worktree]{WORKTREE:#}")
            }
        }
    }
}
