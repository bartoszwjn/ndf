use std::{iter, path::Path, process::Command};

pub(crate) fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
}

pub(crate) fn run(command: &mut Command) -> eyre::Result<()> {
    let cmd = iter::once(command.get_program())
        .chain(command.get_args())
        .fold(String::new(), |acc, arg| {
            if acc.is_empty() {
                arg.to_string_lossy().into_owned()
            } else {
                acc + " " + arg.to_string_lossy().as_ref()
            }
        });
    const BOLD: &str = "\u{1b}[1m";
    const RESET: &str = "\u{1b}[0m";
    eprintln!("{BOLD}{cmd}{RESET}");

    let status = command.status()?;
    if !status.success() {
        eyre::bail!("command failed ({status})");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_root_is_correct() {
        let root = super::workspace_root();
        for elem in ["Cargo.toml", "src", "crates", "rust-toolchain.toml"] {
            assert!(
                root.join(elem).try_exists().unwrap(),
                "{elem} file not found"
            );
        }
    }
}
