use clap::Parser;

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
        println!("Hello, world!");
        Ok(())
    }
}
