use std::path::{Path, PathBuf};

use eyre::bail;

use crate::{
    cli::DiffTool,
    color::{BLUE, BOLD, CYAN, MAGENTA},
};

#[derive(Clone, Debug)]
pub(crate) struct DiffSpec {
    pub(crate) source: Source,
    /// Absolute, canonicalized path to repository root.
    pub(crate) repo: PathBuf,
    pub(crate) from: Revision,
    pub(crate) to: Revision,
    pub(crate) tool: DiffTool,
    pub(crate) base: Option<AttrPath>,
    pub(crate) attr_paths: Vec<AttrPath>,
}

#[derive(Clone, Debug)]
pub(crate) enum Source {
    Flake(FlakePath),
    /// Absolute, canonicalized path to the file.
    File(PathBuf),
}

/// Absolute, canonicalized path to the directory containing the `flake.nix` file.
///
/// Guaranteed to contain only characters that can be used in path-like flake references.
#[derive(Clone, Debug)]
pub(crate) struct FlakePath(String);

#[derive(Clone, Debug)]
pub(crate) enum Revision {
    GitRevision { commit_id: String, display: String },
    GitWorktree,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct AttrPath(pub(crate) String);

impl FlakePath {
    pub(crate) fn new(path: PathBuf) -> eyre::Result<Self> {
        assert!(path.is_absolute());
        let string = match path.into_os_string().into_string() {
            Ok(string) => string,
            Err(os_string) => bail!("flake path contains invalid Unicode: {os_string:?}"),
        };
        if let Some(invalid) = string.chars().find(|&c| !Self::is_valid_char(c)) {
            bail!(
                "flake path contains an invalid character: {}",
                invalid.escape_default(),
            )
        }
        Ok(Self(string))
    }

    fn is_valid_char(c: char) -> bool {
        // Nix allows all unicode characters except `#` and `?`, but Lix is more restrictive:
        // https://nix.dev/manual/nix/2.33/command-ref/new-cli/nix3-flake.html#path-like-syntax
        // https://git.lix.systems/lix-project/lix/src/commit/2.94.0/lix/libexpr/flake/flakeref.cc#L86
        //
        // TODO: it should be possible to express any path using URL-like syntax
        // with percent encoding.
        c.is_ascii_alphanumeric() || "-._~!$&'\"()*+,;=/".contains(c)
    }

    pub(crate) fn as_path(&self) -> &Path {
        self.0.as_ref()
    }

    pub(crate) fn as_str(&self) -> &str {
        self.0.as_ref()
    }
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
        macro_rules! header {
            ($name:expr) => {
                format_args!("{BOLD}{: <6}{BOLD:#}", format!("{}:", $name))
            };
        }

        let (source_header, source_path) = match &self.source {
            Source::Flake(flake_path) => ("Flake", flake_path.as_path()),
            Source::File(path) => ("File", path.as_path()),
        };
        writeln!(
            f,
            "{} {BLUE}{}{BLUE:#}",
            header!(source_header),
            source_path.display()
        )?;

        writeln!(f, "{} {}", header!("Repo"), self.repo.display())?;
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

impl std::fmt::Display for Revision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitRevision { display, .. } => f.write_str(display),
            // NOTE: ref names cannot contain '[', see `git check-ref-format --help`.
            Self::GitWorktree => {
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
