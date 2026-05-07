use clap::Parser;

mod common;
mod examples;

fn main() -> eyre::Result<()> {
    let app = XtaskApp::parse();
    color_eyre::install()?;
    app.exec()
}

/// Custom commands used for development.
#[derive(clap::Parser, Debug)]
struct XtaskApp {
    #[command(subcommand)]
    subcommand: Subcommand,
}

impl XtaskApp {
    fn exec(self) -> eyre::Result<()> {
        match self.subcommand {
            Subcommand::Examples(examples_args) => examples_args.exec(),
        }
    }
}

#[derive(clap::Subcommand, Debug)]
enum Subcommand {
    Examples(examples::Args),
}
