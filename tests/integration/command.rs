use std::{
    ops::RangeBounds,
    process::{Command, Output},
};

use color_eyre::{Section, SectionExt};

pub(crate) fn run(cmd: &mut Command) -> eyre::Result<Output> {
    run_allow_exit_codes(cmd, 0..=0)
}

pub(crate) fn run_allow_exit_codes(
    cmd: &mut Command,
    allowed_exit_codes: impl RangeBounds<i32>,
) -> eyre::Result<Output> {
    let output = cmd.output()?;
    if !output
        .status
        .code()
        .is_some_and(|code| allowed_exit_codes.contains(&code))
    {
        let program = cmd.get_program();
        let command = Vec::from_iter(std::iter::chain([program], cmd.get_args()));
        return Err(
            eyre::eyre!("{} command failed ({})", program.display(), output.status)
                .section(format!("{command:?}",).header("Command:"))
                .section(
                    String::from_utf8_lossy(&output.stdout)
                        .into_owned()
                        .header("Captured stdout:"),
                )
                .section(
                    String::from_utf8_lossy(&output.stderr)
                        .into_owned()
                        .header("Captured stderr:"),
                ),
        );
    }
    Ok(output)
}
