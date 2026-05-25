//! Utilities for displaying things to the user.

use std::{ffi::OsStr, fmt};

pub(crate) fn display_command_args<Iter>(make_args: impl Fn() -> Iter) -> impl fmt::Display
where
    Iter: Iterator,
    Iter::Item: AsRef<OsStr>,
{
    fmt::from_fn(move |f| {
        let mut first = true;
        for arg in make_args() {
            let arg = display_command_arg(arg);
            if first {
                first = false;
                write!(f, "{arg}")?;
            } else {
                write!(f, " {arg}")?;
            }
        }
        Ok(())
    })
}

pub(crate) fn display_command_arg(arg: impl AsRef<OsStr>) -> impl fmt::Display {
    fn needs_quoting(c: char) -> bool {
        match c {
            '"' | '\'' | '\\' => true,
            _ if c.is_whitespace() => true,

            _ if c.is_alphanumeric() => false,
            _ if c.is_ascii_punctuation() => false,

            _ => true,
        }
    }

    fmt::from_fn(move |f| {
        let arg = arg.as_ref().to_string_lossy();
        if arg.is_empty() || arg.chars().any(needs_quoting) {
            write!(f, "{arg:?}")
        } else {
            write!(f, "{arg}")
        }
    })
}
