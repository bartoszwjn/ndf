//! Utilities for displaying things to the user.

use std::{ffi::OsStr, fmt};

pub(crate) fn display_command_args<Iter>(make_args: impl Fn() -> Iter) -> impl fmt::Display
where
    Iter: Iterator,
    Iter::Item: AsRef<OsStr>,
{
    fmt::from_fn(move |f| {
        let mut args = make_args().peekable();
        while let Some(arg) = args.next() {
            write!(f, "{}", display_command_arg(arg))?;
            if args.peek().is_some() {
                write!(f, " ")?;
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
