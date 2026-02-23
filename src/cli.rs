use std::{num::NonZero, path::PathBuf, process::ExitCode};

use anstream::{print, println};
use anyhow::Context;
use clap::{Parser, ValueEnum};
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::{diff_spec::DiffSpec, eval};

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
    pub(crate) attr_paths: Vec<String>,

    /// Report changes from this revision.
    ///
    /// When omitted defaults to:
    /// - HEAD if '--base' is not specified,
    /// - current worktree otherwise.
    #[arg(short = 'f', long, verbatim_doc_comment)]
    pub(crate) from: Option<String>,

    /// Report changes to this revision.
    ///
    /// When omitted defaults to the current worktree.
    #[arg(short = 't', long)]
    pub(crate) to: Option<String>,

    /// Compare all other attribute paths to this one.
    #[arg(long)]
    pub(crate) base: Option<String>,

    /// Program used for comparing derivations.
    #[arg(long, default_value = "none")]
    pub(crate) tool: DiffTool,

    /// Interpret paths as attribute paths relative to the Nix expression in the given file.
    #[arg(long)]
    pub(crate) file: Option<PathBuf>,

    /// Interpret paths as attribute paths relative to the given flake reference.
    ///
    /// The default is to interpret paths as relative to the flake located in the current
    /// directory.
    #[arg(long, conflicts_with("file"))]
    pub(crate) flake: Option<String>,

    /// Interpret paths as attribute paths pointing to NixOS configurations.
    ///
    /// Each '<ATTR_PATH>' will be treated
    /// as if '<ATTR_PATH>.config.system.build.toplevel' was passed instead
    /// ('nixosConfigurations.<ATTR_PATH>.config.system.build.toplevel' for flake outputs).
    #[arg(long)]
    pub(crate) nixos: bool,

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
    pub fn run(self) -> anyhow::Result<ExitCode> {
        let eval_jobs = self.eval_jobs;
        let spec = DiffSpec::from_args(self)?;

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
}

fn build_thread_pool(eval_jobs: isize) -> anyhow::Result<Option<ThreadPool>> {
    let num_threads: NonZero<usize> = match eval_jobs {
        1.. => {
            NonZero::new(usize::try_from(eval_jobs).expect("positive isize must fit into usize"))
                .expect("value is positive")
        }
        ..=0 => {
            let available: usize = std::thread::available_parallelism()
                .context("failed to query the number of available threads")?
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
                .context("failed to initialize a thread pool")
                .map(Some)
        }
    }
}
