use std::{
    num::NonZero,
    path::{Path, PathBuf},
    process::ExitCode,
};

use anstream::{print, println};
use clap::{Parser, ValueEnum};
use eyre::WrapErr;
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::{
    diff_spec::{AttrPath, DiffSpec, GitRev, Source},
    eval, git, nix,
};

const AFTER_HELP: &str = concat![
    "Exit code is 0 if all derivations are the same, 1 if any are different,",
    " and something other than 0 or 1 in case of an error.",
];

/// Compare Nix derivations between two revisions.
#[derive(Clone, Debug, Parser)]
#[command(version, after_help(AFTER_HELP))]
pub struct Cli {
    /// Attribute paths to compare.
    ///
    /// Each path is compared between the revisions specified with `--from` and `--to`.
    ///
    /// By default, these paths are interpreted as flake output attributes.
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

    /// Interpret paths as attribute paths relative to the Nix expression in the given file.
    #[arg(long)]
    file: Option<PathBuf>,

    /// Interpret paths as attribute paths relative to the given flake reference.
    ///
    /// The default is to interpret paths as relative to the flake located in the current
    /// directory.
    #[arg(long, conflicts_with("file"))]
    flake: Option<String>,

    /// Interpret paths as attribute paths pointing to NixOS configurations.
    ///
    /// Each '<ATTR_PATH>' will be treated
    /// as if '<ATTR_PATH>.config.system.build.toplevel' was passed instead
    /// ('nixosConfigurations.<ATTR_PATH>.config.system.build.toplevel' for flake outputs).
    #[arg(long)]
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
        let source = match (self.file, self.flake) {
            (None, None) => Source::FlakeCurrentDir,
            (None, Some(_)) => todo!("--flake"),
            (Some(path), None) => Source::File(path),
            (Some(_), Some(_)) => unreachable!("--file and --flake are mutually exclusive"),
        };

        let from = make_from(self.from, &source, &self.base)?;
        let to = make_to(self.to, &source)?;
        let tool = self.tool;

        let base = self
            .base
            .map(|base| attr_path_from_args(base, self.nixos, &source));

        let attr_paths = {
            let attr_paths = if self.attr_paths.is_empty() {
                get_default_attr_paths(&source, self.nixos)?
            } else {
                self.attr_paths
            };
            attr_paths
                .into_iter()
                .map(|attr_path| attr_path_from_args(attr_path, self.nixos, &source))
                .collect()
        };

        Ok(DiffSpec {
            source,
            from,
            to,
            tool,
            base,
            attr_paths,
        })
    }
}

fn make_from(from: Option<String>, source: &Source, base: &Option<String>) -> eyre::Result<GitRev> {
    match (from, base) {
        (Some(from), _) => resolve_git_ref(from, source),
        (None, Some(_)) => Ok(GitRev::Worktree),
        (None, None) => resolve_git_ref("HEAD".to_owned(), source),
    }
}

fn make_to(to: Option<String>, source: &Source) -> eyre::Result<GitRev> {
    match to {
        Some(to) => resolve_git_ref(to, source),
        None => Ok(GitRev::Worktree),
    }
}

fn resolve_git_ref(orig_ref: String, source: &Source) -> eyre::Result<GitRev> {
    let path_in_repo = match source {
        Source::FlakeCurrentDir => Path::new("."),
        Source::File(path) => path.as_path(),
    };
    let rev = git::resolve_ref(&orig_ref, path_in_repo)?;
    Ok(GitRev::Rev { orig_ref, rev })
}

fn get_default_attr_paths(source: &Source, nixos: bool) -> eyre::Result<Vec<String>> {
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
