use std::process::ExitCode;

use anstream::{eprintln, print, println};
use clap::Parser as _;

use crate::{
    cli::{Cli, DiffProgram},
    color::{GREEN_BOLD, RED_BOLD},
    spec::{AttrPath, DiffSpec, GitRev},
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
    let spec = DiffSpec::from_args(args)?;

    println!("{spec}");

    let common_lhs_drv_path = spec
        .common_lhs
        .as_ref()
        .map(|lhs| nix::get_drv_path(&spec.source, &spec.old_rev, lhs))
        .transpose()?;

    let mut summary = Summary { items: vec![] };
    for path in spec.attr_paths {
        let old_drv_path = match &common_lhs_drv_path {
            Some(drv_path) => drv_path.clone(),
            None => nix::get_drv_path(&spec.source, &spec.old_rev, &path)?,
        };
        let new_drv_path = nix::get_drv_path(&spec.source, &spec.new_rev, &path)?;

        match spec.program {
            DiffProgram::None => {}
            DiffProgram::NixDiff => {
                if old_drv_path != new_drv_path {
                    print_pair_cmp(
                        (spec.common_lhs.as_ref().unwrap_or(&path), &spec.old_rev),
                        (&path, &spec.new_rev),
                    );
                    run_nix_diff(&old_drv_path, &new_drv_path)?;
                    println!();
                }
            }
            DiffProgram::Nvd => todo!("nvd diff"),
        }

        summary.items.push(SummaryItem {
            common_lhs: spec.common_lhs.clone(),
            attr_path: path,
            old_drv_path,
            new_drv_path,
        })
    }

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

fn print_pair_cmp(lhs: (&AttrPath, &GitRev), rhs: (&AttrPath, &GitRev)) {
    let width_l = unicode_width::UnicodeWidthStr::width(lhs.0.0.as_str());
    let width_r = unicode_width::UnicodeWidthStr::width(rhs.0.0.as_str());
    let width = width_l.max(width_r);
    let lhs_pad = width - width_l;
    let rhs_pad = width - width_r;
    println!("{RED_BOLD}-{RED_BOLD:#} {}{:lhs_pad$} {}", lhs.0, "", lhs.1);
    println!(
        "{GREEN_BOLD}+{GREEN_BOLD:#} {}{:rhs_pad$} {}",
        rhs.0, "", rhs.1
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
