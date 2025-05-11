use std::process::ExitCode;

use anstream::{eprintln, println};
use clap::Parser as _;

use cli::Cli;
use color::RED_BOLD;
use items::ItemPair;

mod cli;
mod color;
mod command;
mod git;
mod items;
mod nix;

fn main() -> ExitCode {
    let args = Cli::parse();
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{RED_BOLD}error:{RED_BOLD:#} {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Cli) -> anyhow::Result<()> {
    let items = ItemPair::from_args(args)?;

    for pair in items.iter() {
        println!("{}", pair);
    }

    todo!("diff")
}
