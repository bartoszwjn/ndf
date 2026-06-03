use std::{fmt, iter};

use crate::source::Source;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct AttrPath {
    parts: Vec<String>,
    leading_dot: bool,
    nixos: bool,
}

impl AttrPath {
    pub(crate) fn new(leading_dot: bool, parts: Vec<String>, nixos: bool) -> Self {
        Self {
            parts,
            leading_dot,
            nixos,
        }
    }

    pub(crate) fn from_cli_arg(arg: &str, source: &Source, nixos: bool) -> eyre::Result<Self> {
        let mut this = Self::parse_cli_arg(arg)?;
        this.nixos = nixos;

        if this.leading_dot && matches!(source, Source::File(_)) {
            eyre::bail!("attribute paths with leading dots cannot be used together with '--file'")
        }

        Ok(this)
    }

    pub(crate) fn file_query(&self) -> Vec<&str> {
        self.base_query().collect()
    }

    pub(crate) fn flake_query(&self) -> (bool, Vec<&str>) {
        let leading_dot = self.leading_dot || self.nixos;
        let query = if self.nixos && !self.leading_dot {
            iter::chain(["nixosConfigurations"], self.base_query()).collect()
        } else {
            self.base_query().collect()
        };
        (leading_dot, query)
    }

    fn base_query(&self) -> impl Iterator<Item = &str> {
        let suffix = if self.nixos {
            ["config", "system", "build", "toplevel"].as_slice()
        } else {
            &[]
        };
        iter::chain(
            self.parts.iter().map(String::as_str),
            suffix.iter().copied(),
        )
    }

    pub(crate) fn display_width(&self) -> usize {
        let parts_width = if !self.leading_dot && self.parts.is_empty() {
            "(empty)".len()
        } else {
            let num_dots =
                self.parts.len().saturating_sub(1) + if self.leading_dot { 1 } else { 0 };
            let parts_widths = self.parts.iter().map(|part| {
                unicode_width::UnicodeWidthStr::width(part.as_str())
                    + if Self::part_needs_quotes(part) { 2 } else { 0 }
            });
            num_dots + parts_widths.sum::<usize>()
        };

        let suffix_width = if self.nixos { " (NixOS)".len() } else { 0 };

        parts_width + suffix_width
    }

    /// Pretty-print the attr path to the user.
    pub(crate) fn display(&self) -> impl fmt::Display {
        use crate::styles::{ATTR_PATH, ATTR_PATH_NIXOS, ATTR_PATH_QUOTED};

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

            if self.nixos {
                write!(f, " {ATTR_PATH_NIXOS}(NixOS){ATTR_PATH_NIXOS:#}")?;
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
            return Ok(Self::new(leading_dot, parts, false));
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

        Ok(Self::new(leading_dot, parts, false))
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
