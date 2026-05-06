use std::{
    ffi::OsStr,
    fmt,
    ops::RangeBounds,
    process::{Command, Output, Stdio},
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

    pub(crate) fn run_for_exit_code(
        &mut self,
        allowed_exit_codes: impl RangeBounds<i32>,
    ) -> eyre::Result<i32> {
        let output = self.get_output(allowed_exit_codes)?;
        Ok(output
            .status
            .code()
            .expect("exit code is in allowed range, so it's not None"))
    }

    pub(crate) fn output(&mut self) -> eyre::Result<Vec<u8>> {
        Ok(self.get_output(0..=0)?.stdout)
    }

    pub(crate) fn output_json<T: DeserializeOwned>(&mut self) -> eyre::Result<T> {
        let output = self.get_output(0..=0)?;
        serde_json::from_slice(&output.stdout)
            .wrap_err_with(|| {
                format!(
                    "failed to decode output of external program {}",
                    display_program(&self.inner),
                )
            })
            .with_context_from_cmd(self)
            .with_context_from_output(&output)
    }

    fn get_output(&mut self, allowed_exit_codes: impl RangeBounds<i32>) -> eyre::Result<Output> {
        trace_program(&self.inner);
        let output = self
            .inner
            .output()
            .wrap_err_with(|| {
                format!(
                    "failed to execute external program {}",
                    display_program(&self.inner),
                )
            })
            .with_context_from_cmd(self)?;

        let success = output
            .status
            .code()
            .is_some_and(|code| allowed_exit_codes.contains(&code));
        if !success {
            return Err(eyre!(
                "external program {} did not finish successfully ({})",
                display_program(&self.inner),
                output.status,
            )
            .with_context_from_cmd(self)
            .with_context_from_output(&output));
        }

        Ok(output)
    }
}

fn trace_program(command: &Command) {
    tracing::trace!(
        program = %command.get_program().display(),
        args = %display_args(command),
        "executing external program",
    );
}

fn display_command(command: &Command) -> impl fmt::Display {
    display_args_iter(|| {
        [command.get_program()]
            .into_iter()
            .chain(command.get_args())
    })
}

fn display_program(command: &Command) -> impl fmt::Display {
    display_arg(command.get_program())
}

fn display_args(command: &Command) -> impl fmt::Display {
    display_args_iter(|| command.get_args())
}

fn display_args_iter<'a, Iter>(make_iter: impl Fn() -> Iter) -> impl fmt::Display
where
    Iter: Iterator<Item = &'a OsStr>,
{
    fmt::from_fn(move |f| {
        let mut first = true;
        for arg in make_iter() {
            let arg = display_arg(arg);
            if first {
                write!(f, "{arg}")?;
                first = false;
            } else {
                write!(f, " {arg}")?;
            }
        }
        Ok(())
    })
}

fn display_arg(arg: &OsStr) -> impl fmt::Display {
    fn needs_quoting(c: char) -> bool {
        match c {
            '"' | '\'' | '\\' => true,
            _ if c.is_whitespace() => true,

            _ if c.is_alphanumeric() => false,
            _ if c.is_ascii_punctuation() => false,

            _ => true,
        }
    }

    let arg = arg.to_string_lossy();
    fmt::from_fn(move |f| {
        if arg.is_empty() || arg.chars().any(needs_quoting) {
            write!(f, "{arg:?}")
        } else {
            write!(f, "{arg}")
        }
    })
}

trait ContextExt {
    type Return;

    fn with_context_from_cmd(self, cmd: &Cmd) -> Self::Return;
    fn with_context_from_output(self, output: &Output) -> Self::Return;
}

impl<T: Section<Return = T>> ContextExt for T {
    type Return = T::Return;

    fn with_context_from_cmd(self, cmd: &Cmd) -> Self::Return {
        self.with_section(|| display_command(&cmd.inner).to_string().header("Command:"))
    }

    fn with_context_from_output(self, output: &Output) -> Self::Return {
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
