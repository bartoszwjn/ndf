use std::{
    ffi::{OsStr, OsString},
    fs,
    num::NonZero,
    path::{Path, PathBuf},
    process::ExitCode,
};

use anstream::{print, println};
use clap::{Parser, ValueEnum};
use eyre::{WrapErr, bail, eyre};
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::{
    diff_spec::{AttrPath, DiffSpec, FlakePath, GitRev, Source},
    eval, git, nix,
};

const AFTER_HELP: &str = "\
    Exit code is 0 if all derivations are the same, 1 if any are different, \
    and something other than 0 or 1 in case of an error.\
";

/// Compare Nix derivations between two revisions.
#[derive(Clone, Debug, Parser)]
#[command(version, after_help(AFTER_HELP))]
pub struct Cli {
    /// Attribute paths to compare.
    ///
    /// Each path is compared between the revisions specified with `--from` and `--to`.
    ///
    /// By default, these paths are interpreted as flake output attributes
    /// of the flake in the current working directory.
    #[arg()]
    attr_paths: Vec<String>,

    /// Report changes from this revision.
    ///
    /// When omitted defaults to:
    /// - HEAD if '--base' is not specified,
    /// - current worktree otherwise.
    #[arg(short = 'f', long, verbatim_doc_comment)]
    from: Option<String>,

    /// Report changes to this revision.
    ///
    /// When omitted defaults to the current worktree.
    #[arg(short = 't', long)]
    to: Option<String>,

    /// Compare all other attribute paths to this one.
    #[arg(long)]
    base: Option<String>,

    /// Program used for comparing derivations.
    #[arg(long, default_value = "none")]
    tool: DiffTool,

    /// Interpret paths as attribute paths relative to the flake at the given path.
    ///
    /// Only local filesystem paths are supported,
    /// other flake reference types (e.g. 'github') are not.
    #[arg(long, default_value = ".")]
    flake: OsString,

    /// Interpret paths as attribute paths relative to the Nix expression in the given file.
    #[arg(long, conflicts_with("flake"))]
    file: Option<OsString>,

    /// Interpret paths as attribute paths pointing to NixOS configurations.
    ///
    /// Each '<ATTR_PATH>' will be transformed to:
    /// - '<ATTR_PATH>.config.system.build.toplevel' if '--file' was used,
    /// - 'nixosConfigurations.<ATTR_PATH>.config.system.build.toplevel' for flake outputs.
    #[arg(long, verbatim_doc_comment)]
    nixos: bool,

    /// Maximum number of Nix evaluations to perform in parallel.
    ///
    /// Zero (the default) means "as many as there are available threads",
    /// a negative number '-N' means "N fewer than the number of available threads".
    #[arg(long, default_value_t = 0)]
    eval_jobs: isize,
}

/// Program used to compare derivations.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum DiffTool {
    /// Do not diff the derivations, only check if they are identical.
    None,
    /// Use nix-diff to compare derivations.
    NixDiff,
}

impl Cli {
    pub fn run(self) -> eyre::Result<ExitCode> {
        let eval_jobs = self.eval_jobs;
        let spec = self.build_diff_spec()?;

        println!("{spec}");

        let thread_pool = build_thread_pool(eval_jobs)?;
        let summary = eval::eval_and_compare_paths(&spec, thread_pool)?;

        print!("{summary}");

        let all_equal = summary
            .items
            .iter()
            .all(|item| item.old_drv_path == item.new_drv_path);
        Ok(if all_equal {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        })
    }

    fn build_diff_spec(self) -> eyre::Result<DiffSpec> {
        let source = match self.file {
            Some(file) => {
                assert_eq!(self.flake, ".");
                Source::File(
                    validate_file_argument(&file)
                        .wrap_err_with(|| format!("invalid value for option '--file': {file:?}"))?,
                )
            }
            None => Source::Flake(validate_flake_argument(&self.flake).wrap_err_with(|| {
                format!("invalid value for option '--flake': {:?}", self.flake)
            })?),
        };
        let repo = git::get_repo_root(match &source {
            Source::Flake(flake_path) => flake_path.path(),
            Source::File(file_path) => file_path,
        })?;

        Ok(DiffSpec {
            from: make_from(self.from, &self.base, &repo)?,
            to: make_to(self.to, &repo)?,
            tool: self.tool,
            base: self
                .base
                .map(|base| attr_path_from_args(base, self.nixos, &source)),
            attr_paths: {
                let attr_paths = if self.attr_paths.is_empty() {
                    get_default_attr_paths(&source, self.nixos)
                        .wrap_err("failed to determine default attribute paths")?
                } else {
                    self.attr_paths
                };
                attr_paths
                    .into_iter()
                    .map(|attr_path| attr_path_from_args(attr_path, self.nixos, &source))
                    .collect()
            },
            source,
            repo,
        })
    }
}

fn validate_flake_argument(flake: &OsStr) -> eyre::Result<FlakePath> {
    let path = Path::new(flake);
    // Same as the path-like syntax described in `nix flake --help`.
    // https://nix.dev/manual/nix/2.33/command-ref/new-cli/nix3-flake.html#path-like-syntax
    if path.is_relative() && !path.starts_with(Path::new(".")) {
        bail!("flake paths must be absolute paths or start with './'");
    }

    let path = path
        .canonicalize()
        .wrap_err_with(|| format!("failed to resolve path {path:?}"))?;
    let metadata = path
        .metadata()
        .wrap_err_with(|| format!("failed to query metadata of {path:?}"))?;
    if !metadata.is_dir() {
        bail!("{path:?} is not a directory");
    }

    let mut flake_path = path.clone();
    loop {
        flake_path.push("flake.nix");
        let has_flake_nix = flake_path
            .try_exists()
            .wrap_err_with(|| format!("failed to check for existence of {flake_path:?}"))?;
        flake_path.pop();
        if has_flake_nix {
            return FlakePath::new(flake_path);
        }

        flake_path.push(".git");
        let has_dot_git = flake_path
            .try_exists()
            .wrap_err_with(|| format!("failed to check for existence of {flake_path:?}"))?;
        flake_path.pop();
        if has_dot_git || &flake_path == "/" {
            bail!(
                "path {path:?} is not part of a flake \
                (neither it nor its parent directories contain a 'flake.nix' file)"
            );
        }

        flake_path.pop();
    }
}

fn validate_file_argument(file: &OsStr) -> eyre::Result<PathBuf> {
    let s = file.to_string_lossy();

    if file.is_empty() {
        return Err(eyre!("empty paths are not supported"));
    }
    // Reject special forms accepted by `nix`'s `--file` option.
    // https://docs.lix.systems/manual/lix/2.94/command-ref/nix-build.html#fileish-syntax
    if s.starts_with('<') && s.ends_with('>') {
        return Err(eyre!(
            "search paths (paths surrounded by '<' and '>') are not supported"
        ));
    }
    for prefix in ["http://", "https://", "flake:", "channel:"] {
        if s.starts_with(prefix) {
            return Err(eyre!("paths starting with {prefix:?} are not supported"));
        }
    }

    fs::canonicalize(file).wrap_err_with(|| format!("failed to resolve path {file:?}"))
}

fn make_from(
    from: Option<String>,
    base: &Option<String>,
    repo_root: &Path,
) -> eyre::Result<GitRev> {
    match (from, base) {
        (Some(from), _) => resolve_git_commit(from, repo_root),
        (None, Some(_)) => Ok(GitRev::Worktree),
        (None, None) => resolve_git_commit("HEAD".to_owned(), repo_root),
    }
}

fn make_to(to: Option<String>, repo_root: &Path) -> eyre::Result<GitRev> {
    match to {
        Some(to) => resolve_git_commit(to, repo_root),
        None => Ok(GitRev::Worktree),
    }
}

fn resolve_git_commit(commit: String, repo_root: &Path) -> eyre::Result<GitRev> {
    let rev = git::resolve_commit(&commit, repo_root)?;
    Ok(GitRev::Rev {
        orig_ref: commit,
        rev,
    })
}

fn get_default_attr_paths(source: &Source, nixos: bool) -> eyre::Result<Vec<String>> {
    Ok(match source {
        Source::Flake(flake_path) if nixos => nix::get_flake_nixos_configurations(flake_path)?,
        Source::Flake(flake_path) => nix::get_flake_packages(flake_path)?,
        Source::File(file) => nix::get_file_output_attributes(file)?,
    })
}

fn attr_path_from_args(attr_path: String, nixos: bool, source: &Source) -> AttrPath {
    match (nixos, source) {
        (false, _) => AttrPath(attr_path),
        (true, Source::Flake(_)) => {
            let mut attr_path = attr_path;
            attr_path.insert_str(0, "nixosConfigurations.");
            AttrPath(attr_path + ".config.system.build.toplevel")
        }
        (true, Source::File(_)) => AttrPath(attr_path + ".config.system.build.toplevel"),
    }
}

fn build_thread_pool(eval_jobs: isize) -> eyre::Result<Option<ThreadPool>> {
    let num_threads: NonZero<usize> = match eval_jobs {
        1.. => {
            NonZero::new(usize::try_from(eval_jobs).expect("positive isize must fit into usize"))
                .expect("value is positive")
        }
        ..=0 => {
            let available: usize = std::thread::available_parallelism()
                .wrap_err("failed to query the number of available threads")?
                .get();
            log::debug!("Available parallelism: {available}");
            let reduce_by: usize = eval_jobs.unsigned_abs();
            NonZero::new(available.saturating_sub(reduce_by)).unwrap_or(NonZero::<usize>::MIN)
        }
    };

    match num_threads.get() {
        0 => unreachable!(),
        1 => {
            log::debug!("Requested parallelism of 1, using only the current thread");
            Ok(None)
        }
        num_threads @ 2.. => {
            log::debug!("Starting a thread pool with {num_threads} threads");
            ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .wrap_err("failed to initialize a thread pool")
                .map(Some)
        }
    }
}
