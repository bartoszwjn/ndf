use std::process::ExitCode;

use anstream::AutoStream;
use anstyle::{AnsiColor, Style};
use clap::Parser;
use tracing_subscriber::{
    filter::EnvFilter,
    layer::{Layer, SubscriberExt},
    util::SubscriberInitExt,
};

use ndf::NdfApp;

fn main() -> ExitCode {
    match exec() {
        Ok(exit_code) => exit_code,
        // In case of an unwinding panic the exit code is 101.
        // Aborting panic raises SIGABRT (6).
        Err(error) => {
            const RED_BOLD: Style = AnsiColor::Red.on_default().bold();
            anstream::eprintln!("{RED_BOLD}error:{RED_BOLD:#} {error:?}");
            ExitCode::from(2)
        }
    }
}

fn exec() -> eyre::Result<ExitCode> {
    let app = NdfApp::parse(); // on error returns with exit code 2

    init_eyre()?;
    init_tracing(app.default_log_level());

    app.exec()
}

fn init_eyre() -> eyre::Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .display_location_section(false)
        .capture_span_trace_by_default(false)
        .try_into_hooks()?;

    eyre_hook.install()?;
    std::panic::set_hook(Box::new(move |panic_info| {
        anstream::eprintln!("{}", panic_hook.panic_report(panic_info))
    }));

    Ok(())
}

fn init_tracing(default_level: tracing::Level) {
    let env_filter = EnvFilter::builder()
        .with_default_directive(default_level.into())
        .from_env_lossy();

    let color_choice = AutoStream::choice(&std::io::stderr());
    let show_time = tracing::Level::TRACE <= default_level;
    let show_target = tracing::Level::DEBUG <= default_level;

    let fmt = {
        let base = tracing_subscriber::fmt::layer()
            .with_target(show_target)
            .with_ansi_sanitization(false)
            .with_writer(move || AutoStream::new(std::io::stderr().lock(), color_choice));
        if show_time {
            base.boxed()
        } else {
            base.without_time().boxed()
        }
    };

    tracing_subscriber::registry()
        .with(fmt)
        .with(tracing_error::ErrorLayer::default())
        .with(env_filter)
        .init();
}
