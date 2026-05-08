use std::fmt;

use percent_encoding::{AsciiSet, CONTROLS};

use crate::diff_spec::Source;

#[cfg(test)]
mod tests;

#[derive(Clone, Eq, Hash, PartialEq)]
pub(crate) struct AttrPath {
    parts: Vec<String>,
    leading_dot: bool,
}

impl AttrPath {
    pub(crate) fn from_cli_arg(s: &str, source: &Source, nixos: bool) -> eyre::Result<Self> {
        let mut this = Self::parse_cli_arg(s)?;

        if this.leading_dot && matches!(source, Source::File(_)) {
            eyre::bail!("attribute paths with leading dots cannot be used together with '--file'")
        }
        if this.leading_dot && nixos {
            eyre::bail!("attribute paths with leading dots cannot be used together with '--nixos'")
        }

        if nixos {
            this.add_nixos_parts(source);
        }

        Ok(this)
    }

    pub(crate) fn from_parts(parts: Vec<String>) -> Self {
        Self {
            parts,
            leading_dot: false,
        }
    }

    pub(crate) fn from_parts_nixos(parts: Vec<String>, source: &Source) -> Self {
        let mut this = Self::from_parts(parts);
        this.add_nixos_parts(source);
        this
    }

    fn add_nixos_parts(&mut self, source: &Source) {
        match source {
            Source::Flake(_) => self.parts.insert(0, "nixosConfigurations".into()),
            Source::File(_) => {}
        }
        self.parts.extend(
            ["config", "system", "build", "toplevel"]
                .into_iter()
                .map(Into::into),
        );
    }

    /// Display the attr path in a form suitable for using as a command line argument to Nix.
    ///
    /// This can be used with the `-A`/`--attr` option of old-style Nix commands (e.g. `nix-build`),
    /// or the new-style Nix commands (e.g. `nix build`) as the installable argument
    /// when using either `--file` or `--expr` options.
    pub(crate) fn to_cli_arg(&self) -> eyre::Result<impl fmt::Display> {
        if self.parts.iter().any(|part| part.contains('"')) {
            eyre::bail!(
                "attribute paths containing '\"' cannot be passed to Nix on the command line"
            )
        }

        Ok(fmt::from_fn(|f| {
            if self.leading_dot {
                write!(f, ".")?;
            }

            let mut first = true;
            for part in &self.parts {
                if first {
                    first = false;
                } else {
                    write!(f, ".")?;
                }

                if Self::part_needs_quotes(part) {
                    write!(f, "\"{part}\"")?;
                } else {
                    write!(f, "{part}")?;
                }
            }
            Ok(())
        }))
    }

    /// Display the attr path in a from suitable for using as part of a flake reference.
    pub(crate) fn to_flake_fragment(&self) -> eyre::Result<impl fmt::Display> {
        use std::fmt::Write;

        // https://url.spec.whatwg.org/#fragment-percent-encode-set
        const FRAGMENT_PERCENT_ENCODE_SET: AsciiSet =
            CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');

        struct Encoder<'a, 'b>(&'a mut fmt::Formatter<'b>);
        impl Write for Encoder<'_, '_> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                write!(
                    &mut self.0,
                    "{}",
                    percent_encoding::utf8_percent_encode(s, &FRAGMENT_PERCENT_ENCODE_SET)
                )
            }
        }

        self.to_cli_arg()
            .map(|res| fmt::from_fn(move |f| write!(Encoder(f), "{res}")))
    }

    pub(crate) fn display_width(&self) -> usize {
        if !self.leading_dot && self.parts.is_empty() {
            return "(empty)".len();
        }

        let num_dots = self.parts.len().saturating_sub(1) + if self.leading_dot { 1 } else { 0 };
        let parts_widths = self.parts.iter().map(|part| {
            unicode_width::UnicodeWidthStr::width(part.as_str())
                + if Self::part_needs_quotes(part) { 2 } else { 0 }
        });
        num_dots + parts_widths.sum::<usize>()
    }

    /// Pretty-print the attr path to the user.
    pub(crate) fn display(&self) -> impl fmt::Display {
        use crate::styles::{ATTR_PATH, ATTR_PATH_QUOTED};

        fmt::from_fn(|f| {
            if self.leading_dot {
                write!(f, "{ATTR_PATH}.{ATTR_PATH:#}")?;
            } else if self.parts.is_empty() {
                write!(f, "{ATTR_PATH_QUOTED}(empty){ATTR_PATH_QUOTED:#}")?;
            }

            let mut first = true;
            for part in &self.parts {
                if first {
                    first = false;
                } else {
                    write!(f, "{ATTR_PATH}.{ATTR_PATH:#}")?;
                }

                if Self::part_needs_quotes(part) {
                    write!(f, "{ATTR_PATH_QUOTED}\"{part}\"{ATTR_PATH_QUOTED:#}")?;
                } else {
                    write!(f, "{ATTR_PATH}{part}{ATTR_PATH:#}")?;
                }
            }
            Ok(())
        })
    }

    fn part_needs_quotes(part: &str) -> bool {
        part.is_empty() || part.contains(['.', '"']) || part.contains(char::is_whitespace)
    }

    fn parse_cli_arg(s: &str) -> Result<Self, ParseError> {
        // References:
        // https://git.lix.systems/lix-project/lix/src/commit/7831c98a4db589c84cf730db23793afe3fd90f2d/lix/libexpr/attr-path.hh#L23
        // https://git.lix.systems/lix-project/lix/src/commit/7831c98a4db589c84cf730db23793afe3fd90f2d/lix/libexpr/attr-path.cc#L10

        let (leading_dot, s) = match s.strip_prefix(".") {
            Some(rest) => (true, rest),
            None => (false, s),
        };

        if s.is_empty() {
            let parts = Vec::new();
            return Ok(Self { leading_dot, parts });
        }

        let mut parts = Vec::new();
        let mut current = String::new();
        let mut started = false;
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            match c {
                '.' => {
                    if !started {
                        // This cannot be a leading dot, since we already stripped one at the start.
                        return Err(ParseError::ConsecutiveDots);
                    }
                    parts.push(std::mem::take(&mut current));
                    started = false;
                }
                '"' => {
                    started = true;
                    loop {
                        match chars.next() {
                            None => return Err(ParseError::NoClosingQuote),
                            Some('"') => break,
                            Some(c) => current.push(c),
                        }
                    }
                }
                c => {
                    started = true;
                    current.push(c);
                }
            }
        }
        if started {
            parts.push(current);
        } else {
            return Err(ParseError::TrailingDot);
        }

        Ok(Self { leading_dot, parts })
    }
}

impl fmt::Debug for AttrPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.leading_dot {
            write!(f, ".")?;
        }
        let mut first = true;
        for part in &self.parts {
            if first {
                first = false;
                write!(f, "{part:?}")?;
            } else {
                write!(f, ".{part:?}")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ParseError {
    ConsecutiveDots,
    TrailingDot,
    NoClosingQuote,
}

impl std::error::Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::ConsecutiveDots => {
                write!(
                    f,
                    "consecutive dots are not allowed in attribute paths \
                    (empty attribute names must be quoted)"
                )
            }
            ParseError::TrailingDot => {
                write!(
                    f,
                    "trailing dots are not allowed in attribute paths \
                    (empty attribute names must be quoted)"
                )
            }
            ParseError::NoClosingQuote => write!(f, "missing closing quote in attribute path"),
        }
    }
}
