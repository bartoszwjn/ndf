use std::{
    borrow::Cow,
    ffi::OsStr,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Context as _;
use serde::de::DeserializeOwned;

#[derive(Debug)]
pub(crate) struct Cmd {
    inner: Command,
}

/// Constructing commands
impl Cmd {
    fn new(program: &str) -> Self {
        Self {
            inner: Command::new(program),
        }
    }

    pub(crate) fn nix() -> Self {
        Self::new("nix")
    }

    pub(crate) fn git() -> Self {
        Self::new("git")
    }

    pub(crate) fn nix_diff() -> Self {
        Self::new("nix-diff")
    }
}

macro_rules! cmd_wrapper {
    ($fn_name:ident, $arg_name:ident : $arg_type:ty) => {
        pub(crate) fn $fn_name(&mut self, $arg_name: $arg_type) -> &mut Self {
            self.inner.$fn_name($arg_name);
            self
        }
    };
}

/// Builder methods
impl Cmd {
    cmd_wrapper!(arg, arg: impl AsRef<OsStr>);
    cmd_wrapper!(args, args: impl IntoIterator<Item = impl AsRef<OsStr>>);
    cmd_wrapper!(current_dir, dir: impl AsRef<Path>);
    cmd_wrapper!(stdin, cfg: impl Into<Stdio>);
    cmd_wrapper!(stdout, cfg: impl Into<Stdio>);
    cmd_wrapper!(stderr, cfg: impl Into<Stdio>);
}

/// Running commands
impl Cmd {
    pub(crate) fn run_inherit_stdio(&mut self) -> anyhow::Result<()> {
        let output = self
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()?;
        assert_eq!(output.len(), 0);
        Ok(())
    }

    pub(crate) fn output(&mut self) -> anyhow::Result<Vec<u8>> {
        log::debug!("executing command: {}", show_cmd(&self.inner));
        let output = self
            .inner
            .output()
            .with_context(|| format!("failed to run {}", show_cmd(&self.inner)))?;

        if !output.status.success() {
            let mut msg = format!(
                "external command did not finish successfully\ncommand: {}\n{}\n",
                show_cmd(&self.inner),
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

    pub(crate) fn output_json<T: DeserializeOwned>(&mut self) -> anyhow::Result<T> {
        serde_json::from_slice(&self.output()?)
            .with_context(|| format!("failed to decode output of {}", show_cmd(&self.inner)))
    }
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
