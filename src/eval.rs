use std::collections::{HashMap, HashSet};

use rayon::{
    ThreadPool,
    iter::{IntoParallelRefIterator, ParallelIterator},
};

use crate::{
    compare,
    diff_spec::{AttrPath, DiffSpec},
    nix,
    summary::{Summary, SummaryItem},
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct EvalSpec<'spec> {
    pub(crate) commit_id: Option<&'spec str>,
    pub(crate) attr_path: &'spec AttrPath,
}

impl<'spec> EvalSpec<'spec> {
    pub(crate) fn lhs(spec: &'spec DiffSpec, rhs_attr_path: &'spec AttrPath) -> Self {
        EvalSpec {
            commit_id: spec.from.commit_id(),
            attr_path: spec.base.as_ref().unwrap_or(rhs_attr_path),
        }
    }

    pub(crate) fn rhs(spec: &'spec DiffSpec, rhs_attr_path: &'spec AttrPath) -> Self {
        EvalSpec {
            commit_id: spec.to.commit_id(),
            attr_path: rhs_attr_path,
        }
    }

    pub(crate) fn run(&self, spec: &DiffSpec) -> eyre::Result<String> {
        nix::get_drv_path(&spec.source, self.commit_id, self.attr_path)
    }
}

pub(crate) fn eval_and_compare_paths(
    spec: &DiffSpec,
    thread_pool: Option<ThreadPool>,
) -> eyre::Result<Summary> {
    let items = match thread_pool {
        Some(thread_pool) => eval_and_compare_paths_parallel(spec, thread_pool)?,
        None => eval_and_compare_paths_sequential(spec)?,
    };
    Ok(Summary { items })
}

/// Evaluate and compare derivation paths using a single thread.
///
/// Evaluation results are cached,
/// and evaluation is delayed until the result is needed for the first time.
/// That way we don't have to wait for all evalutations before showing the first diff.
fn eval_and_compare_paths_sequential(spec: &DiffSpec) -> eyre::Result<Vec<SummaryItem>> {
    let mut cached_results = HashMap::<EvalSpec, eyre::Result<String>>::new();
    let mut get_drv_path = |eval_spec| {
        let result = cached_results
            .entry(eval_spec)
            .or_insert_with(|| eval_spec.run(spec));
        extract_cached_drv_path_result(result)
    };

    spec.attr_paths
        .iter()
        .map(|attr_path| compare::compare_paths(attr_path, spec, &mut get_drv_path))
        .collect::<Result<Vec<_>, _>>()
}

/// Evaluate derivation paths in parallel, then compare them.
fn eval_and_compare_paths_parallel(
    spec: &DiffSpec,
    thread_pool: ThreadPool,
) -> eyre::Result<Vec<SummaryItem>> {
    // Evaluate everything once, then get results from a map.
    let eval_jobs: HashSet<EvalSpec> = spec
        .attr_paths
        .iter()
        .flat_map(|path| [EvalSpec::lhs(spec, path), EvalSpec::rhs(spec, path)])
        .collect();
    let mut cached_results: HashMap<EvalSpec, eyre::Result<String>> = thread_pool.install(|| {
        eval_jobs
            .par_iter()
            .map(|job| (*job, job.run(spec)))
            .collect()
    });
    let mut get_drv_path = |eval_spec| {
        let result = cached_results
            .get_mut(&eval_spec)
            .expect("all results are precomputed");
        extract_cached_drv_path_result(result)
    };

    spec.attr_paths
        .iter()
        .map(|attr_path| compare::compare_paths(attr_path, spec, &mut get_drv_path))
        .collect::<Result<Vec<_>, _>>()
}

fn extract_cached_drv_path_result(result: &mut eyre::Result<String>) -> eyre::Result<String> {
    match result {
        Ok(drv_path) => Ok(drv_path.clone()),
        Err(error) => {
            // eyre errors can't be cloned, return the error we have saved and leave
            // a placeholder error in case someone tries to get the same result again.
            let placeholder = eyre::eyre!("failed to evaluate derivation path");
            Err(std::mem::replace(error, placeholder))
        }
    }
}
