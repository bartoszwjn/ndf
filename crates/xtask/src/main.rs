use clap::Parser;

mod common;

fn main() -> eyre::Result<()> {
    let app = XtaskApp::parse();
    color_eyre::install()?;
    app.exec()
}

/// Custom commands used for development.
#[derive(clap::Parser, Debug)]
struct XtaskApp {}

impl XtaskApp {
    fn exec(self) -> eyre::Result<()> {
        let root = common::workspace_root();
        println!("Custom commands for the workspace at {}", root.display());
        println!("There are currently no custom commands");
        Ok(())
    }
}
