use std::{error::Error, process::ExitCode};

use clap::Parser as _;
use cli::Cli;

mod cli;

fn main() -> ExitCode {
    let args = Cli::parse();
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::FAILURE
        }
    }
}

type Result<T, E = Box<dyn Error>> = std::result::Result<T, E>;

fn run(args: Cli) -> Result<()> {
    println!("{:#?}", args);
    Ok(())
}
