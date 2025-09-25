use std::path::PathBuf;

use clap::{Parser, ValueEnum};

const AFTER_HELP: &str = concat![
    "Exit code is 0 if all derivations are the same, 1 if any are different,",
    " and something other than 0 or 1 in case of an error.",
];

/// Compare nix packages and derivations across revisions
#[derive(Clone, Debug, Parser)]
#[command(version, after_help(AFTER_HELP))]
pub(crate) struct Cli {
    /// Compare all other attribute paths to this one.
    #[arg(long)]
    pub(crate) lhs: Option<String>,

    /// Original Git revision to compare against.
    ///
    /// When omitted, use HEAD if '--lhs' is not specified, and the current worktree otherwise.
    #[arg(short, long)]
    pub(crate) old: Option<String>,

    /// New Git revision to compare against the old one.
    ///
    /// When omitted, use the current worktree.
    #[arg(short, long)]
    pub(crate) new: Option<String>,

    /// Program to use for comparing derivations.
    #[arg(short, long, default_value = "none")]
    pub(crate) program: DiffProgram,

    /// Interpret paths as attribute paths relative to the Nix expression in the given file.
    #[arg(short, long)]
    pub(crate) file: Option<PathBuf>,

    /// Interpret paths as attribute paths relative to the given flake reference.
    ///
    /// The default is to interpret paths as relative to the flake located in the current
    /// directory.
    #[arg(long, conflicts_with("file"))]
    pub(crate) flake: Option<String>,

    /// Interpret paths as attribute paths pointing to NixOS configurations.
    ///
    /// Each '<ATTR_PATH>' will be treated as if '<ATTR_PATH>.config.system.build.toplevel'
    /// was passed instead ('nixosConfigurations.<ATTR_PATH>.config.system.build.toplevel'
    /// when working with flake outputs).
    #[arg(long)]
    pub(crate) nixos: bool,

    /// Attribute paths to compare.
    ///
    /// Each path is compared to itself between the old and new revision,
    /// unless '--lhs' is specified.
    ///
    /// These are interpreted as flake output attributes, unless overridden by other options.
    #[arg()]
    pub(crate) attr_paths: Vec<String>,
}

/// Program used to compare derivations.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum DiffProgram {
    /// Use nix-diff to compare derivations.
    NixDiff,
    /// Build the derivations and use nvd to compare derivation outputs.
    Nvd,
    /// Do not diff the derivations, only check if they are identical.
    None,
}
