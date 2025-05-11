use std::{borrow::Cow, ffi::OsStr, process::Command};

use anyhow::Context as _;
use serde::de::DeserializeOwned;

pub(crate) fn run_json<T: DeserializeOwned>(cmd: &str, args: &[&str]) -> anyhow::Result<T> {
    output_json(Command::new(cmd).args(args))
}

pub(crate) fn output(command: &mut Command) -> anyhow::Result<Vec<u8>> {
    let output = command
        .output()
        .with_context(|| format!("failed to run {}", show_cmd(command)))?;

    if !output.status.success() {
        let mut msg = format!(
            "external command did not finish successfully\ncommand: {}\n{}\n",
            show_cmd(command),
            output.status,
        );
        if !output.stdout.is_empty() {
            msg.push_str("stdout:\n");
            msg.push_str(&String::from_utf8_lossy(&output.stdout));
            msg.push('\n');
        }
        if !output.stderr.is_empty() {
            msg.push_str("stderr:\n");
            msg.push_str(&String::from_utf8_lossy(&output.stderr));
            msg.push('\n');
        }
        return Err(anyhow::Error::msg(msg));
    }

    Ok(output.stdout)
}

pub(crate) fn output_json<T: DeserializeOwned>(command: &mut Command) -> anyhow::Result<T> {
    serde_json::from_slice(&output(command)?)
        .with_context(|| format!("failed to decode output of {}", show_cmd(command)))
}

fn show_cmd(command: &Command) -> String {
    let cmd = show_arg(command.get_program()).into_owned();
    command
        .get_args()
        .map(show_arg)
        .fold(cmd, |acc, arg| acc + " " + &arg)
}

fn show_arg(arg: &OsStr) -> Cow<'_, str> {
    let arg = arg.to_string_lossy();
    if arg.is_empty() || arg.contains(['"', '\'']) || arg.contains(char::is_whitespace) {
        let mut escaped = arg.replace('\'', "'\\''");
        escaped.insert(0, '\'');
        escaped.push('\'');
        Cow::Owned(escaped)
    } else {
        arg
    }
}
