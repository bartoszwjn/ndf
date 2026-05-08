use std::fmt;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct AttrPath(String);

impl AttrPath {
    pub(crate) fn new(s: String) -> Self {
        Self(s)
    }

    /// Display the attr path in a form suitable for using as a command line argument to Nix.
    ///
    /// This can be used with the `-A`/`--attr` option of old-style Nix commands (e.g. `nix-build`),
    /// or the new-style Nix commands (e.g. `nix build`) as the installable argument
    /// when using either `--file` or `--expr` options.
    pub(crate) fn to_cli_arg(&self) -> impl fmt::Display {
        &self.0
    }

    /// Display the attr path in a from suitable for using as part of a flake reference.
    pub(crate) fn to_flake_fragment(&self) -> impl fmt::Display {
        self.to_cli_arg() // TODO: urlencode
    }

    pub(crate) fn display_width(&self) -> usize {
        unicode_width::UnicodeWidthStr::width(self.0.as_str())
    }

    /// Pretty-print the attr path to the user.
    pub(crate) fn display(&self) -> impl fmt::Display {
        use crate::styles::ATTR_PATH;
        fmt::from_fn(|f| write!(f, "{ATTR_PATH}{}{ATTR_PATH:#}", self.0))
    }
}
