use std::process::ExitCode;

use anstream::{eprintln, println};
use clap::Parser as _;

use crate::{
    cli::{Cli, DiffProgram},
    color::{GREEN_BOLD, RED_BOLD},
    spec::DiffSpec,
};

mod cli;
mod color;
mod command;
mod git;
mod nix;
mod spec;

fn main() -> ExitCode {
    let args = Cli::parse();
    env_logger::init();
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{RED_BOLD}error:{RED_BOLD:#} {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Cli) -> anyhow::Result<()> {
    let spec = DiffSpec::from_args(args)?;

    println!("{spec}");

    let common_lhs_drv_path = spec
        .common_lhs
        .as_ref()
        .map(|lhs| nix::get_drv_path(&spec.source, &spec.old_rev, lhs))
        .transpose()?;

    for path in &spec.attr_paths {
        let old_drv_path = match &common_lhs_drv_path {
            Some(drv_path) => drv_path,
            None => &nix::get_drv_path(&spec.source, &spec.old_rev, path)?,
        };
        let new_drv_path = nix::get_drv_path(&spec.source, &spec.new_rev, path)?;

        println!(
            "{RED_BOLD}-{RED_BOLD:#} {} {}",
            old_drv_path,
            spec.common_lhs.as_ref().unwrap_or(path)
        );
        println!("{GREEN_BOLD}+{GREEN_BOLD:#} {} {}", new_drv_path, path);

        match spec.program {
            DiffProgram::None => {}
            DiffProgram::NixDiff => {
                todo!("nix-diff diff")
            }
            DiffProgram::Nvd => todo!("nvd diff"),
        }
    }

    todo!("summary");
}
