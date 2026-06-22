use std::{ffi::OsString, process::ExitCode};

use eyre::WrapErr;
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::{
    attr_path::AttrPath,
    diff_spec::DiffSpec,
    eval,
    glob::Pattern,
    nix,
    source::Source,
    summary::EvalResultCmp,
    vcs::{Repository, Revision, VcsMode},
};

const AFTER_HELP: &str = "\
    Exit code is 0 if all derivations are the same, 1 if any are different, \
    and something other than 0 or 1 in case of an error.\
";

/// Compare Nix derivations between two revisions.
#[derive(clap::Parser, Debug)]
#[command(version, after_help(AFTER_HELP), max_term_width(100))]
pub struct NdfApp {
    /// Attribute paths to compare.
    ///
    /// Each path is compared between revisions specified with `-r`, or `-f` and `-t`.
    ///
    /// By default, these paths are interpreted as flake output attributes
    /// of the flake in the current working directory.
    #[arg(value_name = "ATTR_PATH")]
    attr_paths: Vec<String>,

    /// Report changes for this revision.
    ///
    /// If `--base` is not provided, show changes in this revision compared to its first parent.
    /// If `--base` is provided, use this revision on both sides of the comparison.
    ///
    /// The special value `[working tree]` means "the Git working tree" (Git mode only).
    /// With `-r [working tree]` and no `--base`, the working tree is compared to `HEAD`.
    ///
    /// If none of `-r`, `-f`, or `-t` is provided,
    /// then the default is `-r [working tree]` in Git mode and `-r @` in Jujutsu mode.
    #[arg(long, short = 'r')]
    revision: Option<String>,

    /// Report changes from this revision.
    ///
    /// The special value `[working tree]` means "the Git working tree" (Git mode only).
    ///
    /// If none of `-r`, `-f`, or `-t` is provided,
    /// then the default is `-r [working tree]` in Git mode and `-r @` in Jujutsu mode.
    ///
    /// If `-t` is provided then the default for `-f` is
    /// `[working tree]` in Git mode and `@` in Jujutsu mode.
    #[arg(long, short = 'f', conflicts_with("revision"))]
    from: Option<String>,

    /// Report changes to this revision.
    ///
    /// The special value `[working tree]` means "the Git working tree" (Git mode only).
    ///
    /// If none of `-r`, `-f`, or `-t` is provided,
    /// then the default is `-r [working tree]` in Git mode and `-r @` in Jujutsu mode.
    ///
    /// If `-f` is provided then the default for `-t` is
    /// `[working tree]` in Git mode and `@` in Jujutsu mode.
    #[arg(long, short = 't', conflicts_with("revision"))]
    to: Option<String>,

    /// Compare all other attribute paths to this one.
    ///
    /// Each comparison will use this attribute path
    /// on the side corresponding to the `--from` revision.
    #[arg(long)]
    base: Option<String>,

    /// Interpret paths as attribute paths relative to the flake at the given path.
    ///
    /// Only local filesystem paths are supported,
    /// other flake reference types (e.g. 'github') are not.
    #[arg(long, default_value = ".")]
    flake: String,

    /// Interpret paths as attribute paths relative to the Nix expression in the given file.
    #[arg(long, conflicts_with("flake"))]
    file: Option<OsString>,

    /// Interpret paths as attribute paths pointing to NixOS configurations.
    ///
    /// When this flag is present:
    /// - Each `<ATTR_PATH>` is treated as if `.config.system.build.toplevel` was appended to it.
    /// - Flake output attribute paths without a leading dot are interpreted as relative to
    ///   the `nixosConfigurations` output.
    /// - The default flake outputs that are compared are the elements of `nixosConfigurations`.
    #[arg(long, verbatim_doc_comment)]
    nixos: bool,

    /// Interpret each `<ATTR_PATH>` as a glob pattern.
    #[arg(long, short = 'g')]
    glob: bool,

    /// Evaluate flake outputs without pure evaluation mode.
    ///
    /// Has no effect when used with `--file`.
    #[arg(long)]
    impure: bool,

    /// Program used for comparing derivations.
    #[arg(long, default_value = "none")]
    tool: DiffTool,

    /// Additional arguments passed to the tool that compares derivations.
    ///
    /// The default value depends on the tool:
    /// - `nix-diff`: `["--skip-already-compared", "--character-oriented"]`
    ///
    /// Note on parsing: after encountering `--tool-extra-args` all further arguments
    /// will be treated as values for this option, until an optional end marker value `;`.
    ///
    /// When mixing this option with other options, either:
    /// - specify `--tool-extra-args` last,
    /// - use `;` to mark where values for `--tool-extra-args` end,
    /// - use `--tool-extra-args=<value>` to pass one value at a time (can be repeated).
    ///
    /// In most shells the `;` argument needs to be quoted.
    #[arg(
        long,
        num_args = 0..,
        allow_hyphen_values = true,
        value_terminator = ";",
        verbatim_doc_comment,
    )]
    tool_extra_args: Option<Vec<String>>,

    /// Use Git to parse and display revisions (Git mode).
    ///
    /// The default is `--jj` if the repository root contains a `.jj` entry, and `--git` otherwise.
    #[arg(long)]
    git: bool,

    /// Use Jujutsu to parse and display revisions (Jujutsu mode).
    ///
    /// The default is `--jj` if the repository root contains a `.jj` entry, and `--git` otherwise.
    #[arg(long, conflicts_with("git"))]
    jj: bool,

    /// Maximum number of Nix evaluations to perform in parallel.
    ///
    /// When set to zero (the default), the number of CPUs in the system will be used.
    ///
    /// When set to a negative number `-N`, the number of CPUs minus `N` will be used.
    #[arg(long, short = 'j', default_value_t = 0)]
    eval_jobs: isize,

    #[command(flatten)]
    logging: LoggingOptions,
}

/// Program used to compare derivations.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub(crate) enum DiffTool {
    /// Do not diff the derivations, only check if they are identical.
    None,
    /// Use nix-diff to compare derivations.
    NixDiff,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Logging options")]
struct LoggingOptions {
    /// Be less verbose.
    ///
    /// Can be specified multiple times, with each instance further reducing log verbosity.
    ///
    /// Each instance of `--quiet` cancels out one instance of `--verbose` and vice versa.
    #[arg(long, short = 'q', action = clap::ArgAction::Count)]
    quiet: u8,

    /// Be more verbose.
    ///
    /// Can be specified multiple times, with each instance further increasing log verbosity.
    ///
    /// Each instance of `--verbose` cancels out one instance of `--quiet` and vice versa.
    #[arg(long, short = 'v', action = clap::ArgAction::Count)]
    verbose: u8,
}

impl NdfApp {
    pub fn exec(self) -> eyre::Result<ExitCode> {
        let eval_jobs = self.eval_jobs;
        let spec = self.build_diff_spec()?;

        anstream::println!("{spec}");

        let thread_pool = build_thread_pool(eval_jobs)?;
        let summary = eval::eval_and_compare_paths(&spec, thread_pool)?;

        anstream::print!("{summary}");

        let exit_code = summary.items.iter().fold(0, |acc, item| {
            acc.max(match item.result_old.compare(&item.result_new) {
                EvalResultCmp::Equal => 0,
                EvalResultCmp::NotEqual => 1,
                EvalResultCmp::Unknown => 2,
            })
        });

        Ok(ExitCode::from(exit_code))
    }

    pub fn default_log_level(&self) -> tracing::Level {
        match i16::from(self.logging.verbose) - i16::from(self.logging.quiet) {
            ..=-2 => tracing::Level::ERROR,
            -1 => tracing::Level::WARN,
            0 => tracing::Level::INFO,
            1 => tracing::Level::DEBUG,
            2.. => tracing::Level::TRACE,
        }
    }

    fn build_diff_spec(self) -> eyre::Result<DiffSpec> {
        let source = if let Some(file) = &self.file {
            assert_eq!(self.flake, ".");
            Source::file(file)
                .wrap_err_with(|| format!("invalid value for option '--file': {file:?}"))?
        } else {
            let flake = &self.flake;
            Source::flake(flake)
                .wrap_err_with(|| format!("invalid value for option '--flake': {flake:?}"))?
        };

        let vcs_mode_override = match (self.git, self.jj) {
            (false, false) => None,
            (true, false) => Some(VcsMode::Git),
            (false, true) => Some(VcsMode::Jujutsu),
            (true, true) => unreachable!("--git and --jj are mutually exclusive"),
        };
        let repo = Repository::for_source(&source, vcs_mode_override)?;
        let (from, to) = self.make_from_and_to(&repo)?;

        let base = self
            .base
            .as_deref()
            .map(|base| AttrPath::from_cli_arg(base, &source, self.nixos))
            .transpose()
            .wrap_err_with(|| format!("invalid value for option '--base': {:?}", self.base))?;

        let attr_paths = get_attr_paths(
            PartialSpec {
                repo: &repo,
                source: &source,
                from: &from,
                to: &to,
                nixos: self.nixos,
                impure: self.impure,
            },
            &self.attr_paths,
            self.glob,
        )?;

        let tool_extra_args = self
            .tool_extra_args
            .unwrap_or_else(|| default_tool_args(self.tool));

        Ok(DiffSpec {
            source,
            repo,
            from,
            to,
            impure: self.impure,
            tool: self.tool,
            tool_extra_args,
            base,
            attr_paths,
        })
    }

    fn make_from_and_to(&self, repo: &Repository) -> eyre::Result<(Revision, Revision)> {
        let default_rev = match repo.mode() {
            VcsMode::Git => "[working tree]",
            VcsMode::Jujutsu => "@",
        };
        match (&self.revision, &self.from, &self.to) {
            (revision, None, None) => {
                let rev = revision.as_deref().unwrap_or(default_rev);
                if self.base.is_none() {
                    let to = repo.resolve_commit(rev)?;
                    let from = repo.get_first_parent(&to)?;
                    Ok((from, to))
                } else {
                    let from_and_to = repo.resolve_commit(rev)?;
                    Ok((from_and_to.clone(), from_and_to))
                }
            }
            (None, from, to) => {
                let from = from.as_deref().unwrap_or(default_rev);
                let to = to.as_deref().unwrap_or(default_rev);
                Ok((repo.resolve_commit(from)?, repo.resolve_commit(to)?))
            }
            (Some(_), Some(_), _) | (Some(_), _, Some(_)) => {
                unreachable!("--revision is mutually exclusive with --from and --to")
            }
        }
    }
}

fn default_tool_args(tool: DiffTool) -> Vec<String> {
    match tool {
        DiffTool::None => Vec::new(),
        DiffTool::NixDiff => ["--skip-already-compared", "--character-oriented"]
            .map(String::from)
            .into(),
    }
}

#[derive(Clone, Copy, Debug)]
struct PartialSpec<'a> {
    repo: &'a Repository,
    source: &'a Source,
    from: &'a Revision,
    to: &'a Revision,
    nixos: bool,
    impure: bool,
}

fn get_attr_paths(
    spec: PartialSpec,
    attr_paths: &[String],
    glob: bool,
) -> eyre::Result<Vec<AttrPath>> {
    if attr_paths.is_empty() {
        let names =
            get_default_attr_names(spec).wrap_err("failed to determine default attribute paths")?;
        let paths = names
            .into_iter()
            .map(|name| AttrPath::new(false, vec![name], spec.nixos))
            .collect();
        Ok(paths)
    } else if glob {
        let patterns = attr_paths
            .iter()
            .map(|path| {
                Pattern::from_cli_arg(path, spec.source)
                    .wrap_err_with(|| format!("invalid value for positional argument: {path:?}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        get_matching_output_attrs(spec, &patterns)
    } else {
        // In the other branches Nix fetches the sources when computing attr names.
        prefetch_sources(spec)?;
        attr_paths
            .iter()
            .map(|path| {
                AttrPath::from_cli_arg(path, spec.source, spec.nixos)
                    .wrap_err_with(|| format!("invalid value for positional argument: {path:?}"))
            })
            .collect()
    }
}

fn get_default_attr_names(spec: PartialSpec) -> eyre::Result<Vec<String>> {
    let from_commit = spec.from.commit_id();
    let to_commit = spec.to.commit_id();

    let get_for_commit = |commit_id| match spec.source {
        Source::Flake(flake_path) => {
            nix::get_flake_output_names(flake_path, commit_id, spec.nixos, spec.impure)
        }
        Source::File(file_path) => {
            nix::get_file_output_names(spec.repo.root(), file_path, commit_id)
        }
    };

    let mut names = get_for_commit(from_commit)?;
    if from_commit != to_commit {
        names.extend(get_for_commit(to_commit)?);
        names.sort();
        names.dedup();
    }
    Ok(names)
}

fn get_matching_output_attrs(
    spec: PartialSpec,
    patterns: &[Pattern],
) -> eyre::Result<Vec<AttrPath>> {
    let from_commit = spec.from.commit_id();
    let to_commit = spec.to.commit_id();

    let get_for_commit = |commit_id| match spec.source {
        Source::Flake(flake_path) => nix::get_matching_flake_outputs(
            flake_path,
            commit_id,
            spec.nixos,
            spec.impure,
            patterns,
        ),
        Source::File(file_path) => nix::get_matching_file_outputs(
            spec.repo.root(),
            file_path,
            commit_id,
            spec.nixos,
            patterns,
        ),
    };

    let mut attr_paths_by_pattern = get_for_commit(from_commit)?;
    if from_commit != to_commit {
        let attr_paths_by_pattern_2 = get_for_commit(to_commit)?;
        assert_eq!(attr_paths_by_pattern.len(), attr_paths_by_pattern_2.len());
        for (l, r) in attr_paths_by_pattern
            .iter_mut()
            .zip(attr_paths_by_pattern_2)
        {
            l.extend(r);
        }
    }

    assert_eq!(patterns.len(), attr_paths_by_pattern.len());
    for (pattern, attr_paths) in patterns.iter().zip(&attr_paths_by_pattern) {
        if attr_paths.is_empty() {
            tracing::warn!(%pattern, "pattern did not match any attribute paths");
        }
    }

    let mut attr_paths = attr_paths_by_pattern
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    attr_paths.sort();
    attr_paths.dedup();
    Ok(attr_paths)
}

/// Try to force Nix to fetch the Git revisions that will be used for evaluation.
///
/// This is to avoid `SQLite database '.../fetcher-cache-v1.sqlite' is busy` errors
/// that happen when running too many instances of Lix in parallel (as of 2.95.2).
///
/// This is only a best-effort attempt, since we won't know all the sources Nix will have to fetch
/// without actually performing the evaluation.
fn prefetch_sources(spec: PartialSpec) -> eyre::Result<()> {
    let from_commit = spec.from.commit_id();
    let to_commit = spec.to.commit_id();

    let prefetch_commit = |commit| match spec.source {
        Source::Flake(flake_path) => nix::prefetch_flake(flake_path, commit),
        Source::File(_) if let Some(rev) = commit => nix::prefetch_repo(spec.repo.root(), rev),
        Source::File(_) => Ok(()),
    };

    prefetch_commit(from_commit)?;
    if from_commit != to_commit {
        prefetch_commit(to_commit)?
    }
    Ok(())
}

fn build_thread_pool(eval_jobs: isize) -> eyre::Result<Option<ThreadPool>> {
    let num_threads: usize = match eval_jobs {
        positive @ 1_isize.. => positive as usize,
        below_zero @ ..=0 => {
            let available = match std::thread::available_parallelism() {
                Ok(non_zero) => non_zero.get(),
                Err(error) => {
                    tracing::warn!(
                        error = &error as &(dyn std::error::Error + Send + Sync),
                        "failed to query the number of available CPUs, using 1 thread",
                    );
                    1
                }
            };
            available.saturating_add_signed(below_zero).max(1)
        }
    };

    match num_threads {
        0 => unreachable!(),
        1 => {
            tracing::debug!("requested parallelism of 1, using only the current thread");
            Ok(None)
        }
        num_threads @ 2.. => {
            tracing::debug!(num_threads, "starting thread pool");
            ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .wrap_err("thread pool creation failed")
                .map(Some)
        }
    }
}
