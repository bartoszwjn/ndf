use std::{
    borrow::Cow,
    ffi::OsStr,
    path::Path,
    process::{Command, Stdio},
};

use color_eyre::{Section, SectionExt};
use eyre::{WrapErr, eyre};
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
    pub(crate) fn run_inherit_stdio(&mut self) -> eyre::Result<()> {
        let output = self
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()?;
        assert_eq!(output.len(), 0);
        Ok(())
    }

    pub(crate) fn output(&mut self) -> eyre::Result<Vec<u8>> {
        Ok(self.get_output()?.stdout)
    }

    pub(crate) fn output_json<T: DeserializeOwned>(&mut self) -> eyre::Result<T> {
        let output = self.get_output()?;
        serde_json::from_slice(&output.stdout)
            .wrap_err("failed to decode output of external command")
            .with_context_from_cmd(self)
            .with_context_from_output(&output)
    }

    fn get_output(&mut self) -> eyre::Result<std::process::Output> {
        log::debug!("executing command: {}", show_cmd(&self.inner));
        let output = self
            .inner
            .output()
            .wrap_err("failed to run external command")
            .with_context_from_cmd(self)?;

        if !output.status.success() {
            return Err(eyre!(
                "external command did not finish successfully ({})",
                output.status,
            )
            .with_context_from_cmd(self)
            .with_context_from_output(&output));
        }

        Ok(output)
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

trait ContextExt {
    type Return;

    fn with_context_from_cmd(self, cmd: &Cmd) -> Self::Return;
    fn with_context_from_output(self, output: &std::process::Output) -> Self::Return;
}

impl<T: Section<Return = T>> ContextExt for T {
    type Return = T::Return;

    fn with_context_from_cmd(self, cmd: &Cmd) -> Self::Return {
        self.with_section(|| show_cmd(&cmd.inner).header("Command:"))
    }

    fn with_context_from_output(self, output: &std::process::Output) -> Self::Return {
        self.with_section(|| {
            String::from_utf8_lossy(&output.stdout)
                .into_owned()
                .header("Captured stdout:")
        })
        .with_section(|| {
            String::from_utf8_lossy(&output.stderr)
                .into_owned()
                .header("Captured stderr:")
        })
    }
}
