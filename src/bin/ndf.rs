use std::process::ExitCode;

use anstream::eprintln;
use anstyle::{AnsiColor, Style};
use clap::Parser;
use ndf::Cli;

const RED_BOLD: Style = AnsiColor::Red.on_default().bold();

fn main() -> ExitCode {
    match run() {
        Ok(exit_code) => exit_code,
        // In case of an unwinding panic the exit code is 101.
        // Aborting panic raises SIGABRT (6).
        Err(err) => {
            eprintln!("{RED_BOLD}error:{RED_BOLD:#}{err:?}");
            ExitCode::from(2)
        }
    }
}

fn run() -> eyre::Result<ExitCode> {
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .install()?;
    env_logger::init();

    let args = Cli::parse(); // on error returns with exit code 2
    args.run()
}
