use std::process::ExitCode;

use clap::Parser as _;
use cli::Cli;
use items::{Item, ItemPair};

mod cli;
mod command;
mod items;
mod nix;

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

fn run(args: Cli) -> anyhow::Result<()> {
    let items = ItemPair::from_args(args)?;

    for pair in items.iter() {
        fn format_item(item: &Item) -> String {
            format!(
                "src: {:?}, attr: {:?}, ref: {:?}",
                item.source, item.attr_path, item.git_ref
            )
        }
        println!("- {}\n  {}", format_item(&pair.old), format_item(&pair.new));
    }

    Ok(())
}
