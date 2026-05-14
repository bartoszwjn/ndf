use std::process::Command;

use color_eyre::{Section, SectionExt};

pub(crate) fn get_current_system() -> eyre::Result<String> {
    let output = Command::new("nix-instantiate")
        .args([
            "--eval",
            "--strict",
            "--raw",
            "--expr",
            "builtins.currentSystem",
        ])
        .output()?;
    if !output.status.success() {
        return Err(
            eyre::eyre!("nix-instantiate command failed ({})", output.status).section(
                String::from_utf8_lossy(&output.stderr)
                    .into_owned()
                    .header("Captured stderr:"),
            ),
        );
    }
    Ok(String::from_utf8(output.stdout)?)
}
