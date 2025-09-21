use std::process::ExitCode;

use anstream::{eprintln, println};
use clap::Parser as _;

use cli::{Cli, DiffProgram};
use color::{GREEN_BOLD, RED_BOLD};
use items::ItemPair;

mod cli;
mod color;
mod command;
mod git;
mod items;
mod nix;

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
    let program = args.program;
    let items = ItemPair::from_args(args)?;

    for pair in &items {
        println!("{}", pair);
    }
    println!();

    for pair in &items {
        let old_drv_path = nix::get_drv_path(&pair.old)?;
        let new_drv_path = nix::get_drv_path(&pair.new)?;
        println!("{RED_BOLD}-{RED_BOLD:#} {} {}", old_drv_path, pair.old);
        println!("{GREEN_BOLD}+{GREEN_BOLD:#} {} {}", new_drv_path, pair.new);
        match program {
            DiffProgram::None => {}
            DiffProgram::NixDiff => {
                todo!("nix-diff diff")
            }
            DiffProgram::Nvd => todo!("nvd diff"),
        }
    }

    todo!("summary");
}
