use std::{
    collections::{HashMap, HashSet},
    num::NonZero,
    process::ExitCode,
};

use anstream::{eprintln, print, println};
use anyhow::Context as _;
use clap::Parser as _;
use rayon::{
    ThreadPool, ThreadPoolBuilder,
    iter::{IntoParallelRefIterator as _, ParallelIterator as _},
};

use crate::{
    cli::{Cli, DiffProgram},
    color::{GREEN_BOLD, RED_BOLD},
    spec::{AttrPath, DiffSpec, GitRev, Source},
    summary::{Summary, SummaryItem},
};

mod cli;
mod color;
mod command;
mod git;
mod nix;
mod spec;
mod summary;

fn main() -> ExitCode {
    let args = Cli::parse(); // on error returns with exit code 2
    env_logger::init();
    match run(args) {
        Ok(exit_code) => exit_code,
        // In case of an unwinding panic the exit code is 101.
        // Aborting panic raises SIGABRT (6).
        Err(err) => {
            eprintln!("{RED_BOLD}error:{RED_BOLD:#} {err}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Cli) -> anyhow::Result<ExitCode> {
    let eval_jobs = args.eval_jobs;
    let spec = DiffSpec::from_args(args)?;

    println!("{spec}");

    let summary = Summary {
        items: match build_thread_pool(eval_jobs)? {
            Some(thread_pool) => eval_and_compare_paths_parallel(&spec, thread_pool)?,
            None => eval_and_compare_paths_sequential(&spec)?,
        },
    };

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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct EvalSpec<'spec> {
    source: &'spec Source,
    git_rev: &'spec GitRev,
    attr_path: &'spec AttrPath,
}

impl<'spec> EvalSpec<'spec> {
    fn lhs(spec: &'spec DiffSpec, attr_path: &'spec AttrPath) -> Self {
        EvalSpec {
            source: &spec.source,
            git_rev: &spec.old_rev,
            attr_path,
        }
    }

    fn rhs(spec: &'spec DiffSpec, attr_path: &'spec AttrPath) -> Self {
        EvalSpec {
            source: &spec.source,
            git_rev: &spec.new_rev,
            attr_path,
        }
    }

    fn run(&self) -> anyhow::Result<String> {
        nix::get_drv_path(self.source, self.git_rev, self.attr_path)
    }
}

fn eval_and_compare_paths_sequential(spec: &DiffSpec) -> anyhow::Result<Vec<SummaryItem>> {
    // Running on a single thread: cache evaluation results,
    // but delay actual evaluation until the first time we are asked for a result.
    // That way we don't have to wait for all evalutations before showing the first diff.

    let mut cached_results = HashMap::<EvalSpec, anyhow::Result<String>>::new();
    let mut get_drv_path = |eval_spec| {
        let result = cached_results
            .entry(eval_spec)
            .or_insert_with(|| eval_spec.run());
        extract_cached_drv_path_result(result)
    };

    spec.attr_paths
        .iter()
        .map(|attr_path| compare_paths(attr_path, spec, &mut get_drv_path))
        .collect::<Result<Vec<_>, _>>()
}

fn eval_and_compare_paths_parallel(
    spec: &DiffSpec,
    thread_pool: ThreadPool,
) -> anyhow::Result<Vec<SummaryItem>> {
    // Running in parallel: evaluate everything once, then get results from a map.

    let eval_jobs: HashSet<EvalSpec> = {
        let eval_jobs_rhs = spec.attr_paths.iter().map(|path| EvalSpec::rhs(spec, path));
        match &spec.common_lhs {
            Some(lhs) => std::iter::once(EvalSpec::lhs(spec, lhs))
                .chain(eval_jobs_rhs)
                .collect(),
            None => (spec.attr_paths.iter().map(|path| EvalSpec::lhs(spec, path)))
                .chain(eval_jobs_rhs)
                .collect(),
        }
    };
    let mut cached_results: HashMap<EvalSpec, anyhow::Result<String>> =
        thread_pool.install(|| eval_jobs.par_iter().map(|job| (*job, job.run())).collect());
    let mut get_drv_path = |eval_spec| {
        let result = cached_results
            .get_mut(&eval_spec)
            .expect("all results are precomputed");
        extract_cached_drv_path_result(result)
    };

    spec.attr_paths
        .iter()
        .map(|attr_path| compare_paths(attr_path, spec, &mut get_drv_path))
        .collect::<Result<Vec<_>, _>>()
}

fn extract_cached_drv_path_result(result: &mut anyhow::Result<String>) -> anyhow::Result<String> {
    match result {
        Ok(drv_path) => Ok(drv_path.clone()),
        Err(error) => {
            // anyhow errors can't be cloned, return the error we have saved and leave
            // a placeholder error in case someone tries to get the same result again.
            let placeholder = anyhow::anyhow!("failed to evaluate derivation path");
            Err(std::mem::replace(error, placeholder))
        }
    }
}

fn compare_paths<'spec>(
    attr_path: &'spec AttrPath,
    spec: &'spec DiffSpec,
    mut get_drv_path: impl FnMut(EvalSpec<'spec>) -> anyhow::Result<String>,
) -> anyhow::Result<SummaryItem> {
    let attr_path_l = spec.common_lhs.as_ref().unwrap_or(attr_path);
    let lhs_spec = EvalSpec::lhs(spec, attr_path_l);
    let rhs_spec = EvalSpec::rhs(spec, attr_path);
    let old_drv_path = get_drv_path(lhs_spec)?;
    let new_drv_path = get_drv_path(rhs_spec)?;

    match spec.program {
        DiffProgram::None => {}
        DiffProgram::NixDiff => {
            if old_drv_path != new_drv_path {
                print_pair_cmp(lhs_spec, rhs_spec);
                run_nix_diff(&old_drv_path, &new_drv_path)?;
                println!();
            }
        }
        DiffProgram::Nvd => todo!("nvd diff"),
    }

    Ok(SummaryItem {
        common_lhs: spec.common_lhs.clone(),
        attr_path: attr_path.clone(),
        old_drv_path,
        new_drv_path,
    })
}

fn print_pair_cmp(lhs: EvalSpec, rhs: EvalSpec) {
    let width_l = unicode_width::UnicodeWidthStr::width(lhs.attr_path.0.as_str());
    let width_r = unicode_width::UnicodeWidthStr::width(rhs.attr_path.0.as_str());
    let width = width_l.max(width_r);
    let lhs_pad = width - width_l;
    let rhs_pad = width - width_r;
    println!(
        "{RED_BOLD}-{RED_BOLD:#} {}{:lhs_pad$} {}",
        lhs.attr_path, "", lhs.git_rev
    );
    println!(
        "{GREEN_BOLD}+{GREEN_BOLD:#} {}{:rhs_pad$} {}",
        rhs.attr_path, "", rhs.git_rev
    );
}

fn run_nix_diff(old_drv_path: &str, new_drv_path: &str) -> anyhow::Result<()> {
    command::run_inherit_stdio(
        "nix-diff",
        &[
            "--character-oriented",
            "--skip-already-compared",
            old_drv_path,
            new_drv_path,
        ],
    )
}
