use std::path::{Path, PathBuf};

use crate::{
    cli::{Cli, DiffTool},
    color::{BLUE, BOLD, CYAN, GREEN, MAGENTA, YELLOW},
    git, nix,
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
    // TODO: arbitrary flake refs
    File(PathBuf),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum GitRev {
    Rev { orig_ref: String, rev: String },
    Worktree,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct AttrPath(pub(crate) String);

impl DiffSpec {
    pub(crate) fn from_args(args: Cli) -> anyhow::Result<Self> {
        let source = match (args.file, args.flake) {
            (None, None) => Source::FlakeCurrentDir,
            (None, Some(_)) => todo!("--flake"),
            (Some(path), None) => Source::File(path),
            (Some(_), Some(_)) => unreachable!("--file and --flake are mutually exclusive"),
        };

        let from = GitRef::make_from(args.from, &args.base).resolve(&source)?;
        let to = GitRef::make_to(args.to).resolve(&source)?;
        let tool = args.tool;

        let base = args
            .base
            .map(|base| attr_path_from_args(base, args.nixos, &source));

        let attr_paths = {
            let attr_paths = if args.attr_paths.is_empty() {
                get_default_attr_paths(&source, args.nixos)?
            } else {
                args.attr_paths
            };
            attr_paths
                .into_iter()
                .map(|attr_path| attr_path_from_args(attr_path, args.nixos, &source))
                .collect()
        };

        Ok(Self {
            source,
            from,
            to,
            tool,
            base,
            attr_paths,
        })
    }
}

#[derive(Clone, Debug)]
enum GitRef {
    Ref(String),
    Worktree,
}

impl GitRef {
    fn make_from(from: Option<String>, base: &Option<String>) -> Self {
        match (from, base) {
            (Some(from), _) => GitRef::Ref(from),
            (None, Some(_)) => GitRef::Worktree,
            (None, None) => GitRef::Ref("HEAD".to_owned()),
        }
    }

    fn make_to(to: Option<String>) -> Self {
        match to {
            Some(to) => GitRef::Ref(to),
            None => GitRef::Worktree,
        }
    }

    fn resolve(&self, source: &Source) -> anyhow::Result<GitRev> {
        match self {
            Self::Worktree => Ok(GitRev::Worktree),
            Self::Ref(git_ref) => {
                let path_in_repo = match source {
                    Source::FlakeCurrentDir => Path::new("."),
                    Source::File(path) => path.as_path(),
                };
                let rev = git::resolve_ref(git_ref, path_in_repo)?;
                let orig_ref = git_ref.clone();
                Ok(GitRev::Rev { orig_ref, rev })
            }
        }
    }
}

fn get_default_attr_paths(source: &Source, nixos: bool) -> anyhow::Result<Vec<String>> {
    Ok(match source {
        Source::FlakeCurrentDir if nixos => nix::get_current_flake_nixos_configurations()?,
        Source::FlakeCurrentDir => nix::get_current_flake_packages()?,
        Source::File(file) => nix::get_file_output_attributes(file)?,
    })
}

fn attr_path_from_args(attr_path: String, nixos: bool, source: &Source) -> AttrPath {
    match (nixos, source) {
        (false, _) => AttrPath(attr_path),
        (true, Source::FlakeCurrentDir) => {
            let mut attr_path = attr_path;
            attr_path.insert_str(0, "nixosConfigurations.");
            AttrPath(attr_path + ".config.system.build.toplevel")
        }
        (true, Source::File(_)) => AttrPath(attr_path + ".config.system.build.toplevel"),
    }
}

impl std::fmt::Display for DiffSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        macro_rules! header {
            ($name:expr) => {
                format_args!("{BOLD}{: <10}{BOLD:#}", format!("{}:", $name))
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
        writeln!(f, "{}", header!("AttrPaths"))?;
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
