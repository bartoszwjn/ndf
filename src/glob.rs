use std::fmt;

use crate::source::Source;

mod display;
#[cfg(test)]
mod tests;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Pattern {
    parts: Vec<Part>,
    leading_dot: bool,
}

#[derive(Debug, Eq, PartialEq)]
struct Part {
    segments: Vec<Segment>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Segment {
    /// A literal string, matches itself.
    Literal(String),
    /// `?`, matches any single character.
    QuestionMark,
    /// `*`, matches any sequence of characters.
    Star,
    /// `[...]`, matches any single character from the set defined inside the brackets.
    ///
    /// See [`BracketExpr`].
    BracketExpr(BracketExpr),
}

/// `[...]`, matches any single character from the set defined inside the brackets.
///
/// The syntax is almost the same as that of
/// [bracket expressions in extended POSIX regular expressions][posix-re-bracket-expr].
///
/// One exception is that both `^` and `!` can be used to invert the matching behavior when used as
/// the first character following the opening bracket. In extended POSIX regular expressions only
/// `^` is used for that, but POSIX shell globs use `!` instead, and Bash allows either.
///
/// [posix-re-bracket-expr]: https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap09.html#tag_09_03_05
#[derive(Clone, Debug, Eq, PartialEq)]
struct BracketExpr {
    /// Whether the expression starts with `^` or `!`.
    negated: bool,
    /// The contents of the bracket expression, after the optional leading `^` or `!`.
    content: String,
}

impl Pattern {
    pub(crate) fn from_cli_arg(arg: &str, source: &Source) -> eyre::Result<Self> {
        let this = parse_pattern(arg)?;

        if this.leading_dot && matches!(source, Source::File(_)) {
            eyre::bail!("attribute paths with leading dots cannot be used together with '--file'");
        }

        Ok(this)
    }
}

fn parse_pattern(mut s: &str) -> Result<Pattern, ParseError> {
    let leading_dot;
    (leading_dot, s) = match s.strip_prefix('.') {
        Some(rest) => (true, rest),
        None => (false, s),
    };

    let mut parts = Vec::new();
    while !s.is_empty() {
        let (part, rest) = parse_part(s)?;
        parts.push(part);

        if let Some(after_dot) = rest.strip_prefix('.') {
            if after_dot.is_empty() {
                return Err(ParseError::TrailingDot);
            }
            s = after_dot;
        } else {
            assert!(rest.is_empty());
            s = rest;
        }
    }

    Ok(Pattern { leading_dot, parts })
}

fn parse_part(mut s: &str) -> Result<(Part, &str), ParseError> {
    assert!(!s.is_empty());

    let mut started = false;
    let mut segments = Vec::new();
    let mut current_literal = String::new();
    loop {
        let (c, rest) = match next(s) {
            (None, _) => {
                assert!(started);
                push_literal(&mut segments, &mut current_literal);
                return Ok((Part { segments }, s));
            }
            (Some('.'), _) => {
                if !started {
                    // This cannot be a leading dot, since we already stripped one at the start.
                    return Err(ParseError::ConsecutiveDots);
                }
                push_literal(&mut segments, &mut current_literal);
                return Ok((Part { segments }, s));
            }
            (Some(c), rest) => (c, rest),
        };

        started = true;
        match c {
            '"' => {
                let (lit, rest) = parse_quoted_literal(s)?;
                current_literal.push_str(lit);
                s = rest;
            }
            '[' => {
                push_literal(&mut segments, &mut current_literal);
                let (bracket_expr, rest) = parse_bracket_expr(s)?;
                segments.push(Segment::BracketExpr(bracket_expr));
                s = rest;
            }
            '*' => {
                push_literal(&mut segments, &mut current_literal);
                segments.push(Segment::Star);
                if rest.starts_with('*') {
                    return Err(ParseError::ConsecutiveStars);
                }
                s = rest;
            }
            '?' => {
                push_literal(&mut segments, &mut current_literal);
                segments.push(Segment::QuestionMark);
                s = rest;
            }
            c => {
                current_literal.push(c);
                s = rest;
            }
        };
    }
}

fn parse_quoted_literal(s: &str) -> Result<(&str, &str), ParseError> {
    let s = expect(s, '"');
    s.split_once('"').ok_or(ParseError::NoClosingQuote)
}

fn parse_bracket_expr(mut s: &str) -> Result<(BracketExpr, &str), ParseError> {
    s = expect(s, '[');

    let negated;
    (negated, s) = match next(s) {
        (Some('^' | '!'), rest) => (true, rest),
        (Some(_), _) => (false, s),
        (None, _) => return Err(ParseError::UnclosedBracketExpr),
    };

    // All we need to know is where the bracket expression ends,
    // we'll be including its contents into the regex verbatim.
    let content_start = s;
    if let (Some(']'), rest) = next(s) {
        s = rest;
    }
    while let (Some(c), rest) = next(s) {
        match c {
            ']' => {
                let content_len = content_start.len() - s.len();
                let content = &content_start[..content_len];
                let bracket_expr = BracketExpr {
                    negated,
                    content: content.to_owned(),
                };
                return Ok((bracket_expr, rest));
            }

            '[' if let (Some(c @ ('.' | '=' | ':')), rest) = next(rest) => {
                let (end, error) = match c {
                    '.' => (".]", ParseError::UnclosedCollatingSymbol),
                    '=' => ("=]", ParseError::UnclosedEquivalenceClass),
                    ':' => (":]", ParseError::UnclosedCharacterClass),
                    _ => unreachable!(),
                };
                let Some((_, rest)) = rest.split_once(end) else {
                    return Err(error);
                };
                s = rest;
            }

            _ => s = rest,
        }
    }
    Err(ParseError::UnclosedBracketExpr)
}

fn push_literal(segments: &mut Vec<Segment>, literal: &mut String) {
    if !literal.is_empty() {
        segments.push(Segment::Literal(std::mem::take(literal)));
    }
}

fn next(s: &str) -> (Option<char>, &str) {
    let mut chars = s.chars();
    let next = chars.next();
    (next, chars.as_str())
}

fn expect(s: &str, expected: char) -> &str {
    match next(s) {
        (Some(c), rest) if c == expected => rest,
        (Some(c), _) => panic!("input starts with {c:?} when {expected:?} was expected"),
        (None, _) => panic!("end of input when {expected:?} was expected"),
    }
}

#[derive(Debug, Eq, PartialEq)]
enum ParseError {
    ConsecutiveDots,
    TrailingDot,
    NoClosingQuote,
    ConsecutiveStars,
    UnclosedBracketExpr,
    UnclosedCharacterClass,
    UnclosedEquivalenceClass,
    UnclosedCollatingSymbol,
}

impl std::error::Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl ParseError {
    fn message(&self) -> &'static str {
        match self {
            ParseError::ConsecutiveDots => {
                "consecutive dots are not allowed in attribute paths \
                (empty attribute names must be quoted)"
            }
            ParseError::TrailingDot => {
                "trailing dots are not allowed in attribute paths \
                (empty attribute names must be quoted)"
            }
            ParseError::NoClosingQuote => "missing closing quote in attribute path",
            ParseError::ConsecutiveStars => {
                "consecutive wildcards are not allowed in glob patterns \
                (the `**` pattern is not supported)"
            }
            ParseError::UnclosedBracketExpr => "missing closing `]` in pattern",
            ParseError::UnclosedCharacterClass => {
                "missing closing `:]` for a character class inside a bracket expression"
            }
            ParseError::UnclosedEquivalenceClass => {
                "missing closing `=]` for an equivalence class inside a bracket expression"
            }
            ParseError::UnclosedCollatingSymbol => {
                "missing closing `.]` for a collating symbol inside a bracket expression"
            }
        }
    }
}
