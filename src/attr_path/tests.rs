use std::io::Write;

use anstream::StripStream;
use unicode_width::UnicodeWidthStr;

use super::{AttrPath, ParseError};

fn mk(leading_dot: bool, parts: &[&str], nixos: bool) -> AttrPath {
    AttrPath::new(
        leading_dot,
        parts.iter().map(|&s| String::from(s)).collect(),
        nixos,
    )
}

#[test]
fn parse_cli_arg() -> eyre::Result<()> {
    let cases: [(&str, (bool, &[&str])); _] = [
        // simple
        ("a", (false, &["a"])),
        ("foo.bar.baz", (false, &["foo", "bar", "baz"])),
        // leading dot
        (".", (true, &[])),
        (".foo", (true, &["foo"])),
        (".foo.bar", (true, &["foo", "bar"])),
        // quoting
        ("\"foo\"", (false, &["foo"])),
        ("f\"o\"o", (false, &["foo"])),
        ("\".foo\"", (false, &[".foo"])),
        ("\"foo.bar.baz\"", (false, &["foo.bar.baz"])),
        ("foo.\"bar.baz\".quux", (false, &["foo", "bar.baz", "quux"])),
        (
            "foo.\"bar.baz\".quux.\"\".more\".\"dots",
            (false, &["foo", "bar.baz", "quux", "", "more.dots"]),
        ),
        // empty
        ("", (false, &[])),
        ("\"\"", (false, &[""])),
        (".\"\".foo.\"\".bar", (true, &["", "foo", "", "bar"])),
    ];

    for (input, expected) in cases {
        let parsed = AttrPath::parse_cli_arg(input)?;
        let expected = mk(expected.0, expected.1, false);
        assert_eq!(
            expected, parsed,
            "{input:?}: unexpected result of AttrPath::parse_cli_arg",
        );
    }
    Ok(())
}

#[test]
fn parse_cli_arg_errors() {
    let cases = [
        ("..", ParseError::ConsecutiveDots),
        ("..foo.bar", ParseError::ConsecutiveDots),
        ("foo..bar", ParseError::ConsecutiveDots),
        ("foo.bar..", ParseError::ConsecutiveDots),
        ("foo.", ParseError::TrailingDot),
        ("foo.\"bar.baz\".", ParseError::TrailingDot),
        ("\"", ParseError::NoClosingQuote),
        ("foo.\"bar", ParseError::NoClosingQuote),
        ("\"foo.bar\".baz.quux\"", ParseError::NoClosingQuote),
    ];

    for case in cases {
        let result = AttrPath::parse_cli_arg(case.0);
        assert_eq!(
            result,
            Err(case.1),
            "{:?}: unexpected result of AttrPath::parse_cli_arg",
            case.0,
        );
    }
}

#[test]
fn display_width() -> eyre::Result<()> {
    let cases: [&[&str]; _] = [
        &[],
        &[""],
        &["foo"],
        &["foo", "bar", "baz"],
        &["foo.bar.baz"],
        &["foo", "bar.baz", "quux"],
        &["with space"],
    ];

    for parts in cases {
        for leading_dot in [false, true] {
            for nixos in [false, true] {
                let case = mk(leading_dot, parts, nixos);

                let expected_width = case.display_width();

                let mut stream = StripStream::new(Vec::new());
                write!(stream, "{}", case.display())?;
                let output = String::from_utf8(stream.into_inner())?;
                let actual_width = UnicodeWidthStr::width(output.as_str());

                assert_eq!(
                    expected_width, actual_width,
                    "{case:?}: AttrPath::display_width output doesn't match \
                    the actual width of AttrPath::display",
                );
            }
        }
    }
    Ok(())
}

#[test]
fn display() {
    use crate::styles::{ATTR_PATH as A, ATTR_PATH_NIXOS as N, ATTR_PATH_QUOTED as Q};

    type Input<'a> = (bool, &'a [&'a str], bool);
    let cases: [(Input<'_>, String); _] = [
        ((false, &[], false), format!("{Q}(empty){Q:#}")),
        ((true, &[], false), format!("{A}.{A:#}")),
        ((false, &["foo"], false), format!("{A}foo{A:#}")),
        ((true, &["foo"], false), format!("{A}.{A:#}{A}foo{A:#}")),
        (
            (false, &["foo", "bar", "baz"], false),
            format!("{A}foo{A:#}{A}.{A:#}{A}bar{A:#}{A}.{A:#}{A}baz{A:#}"),
        ),
        (
            (true, &["foo", "bar", "baz"], false),
            format!("{A}.{A:#}{A}foo{A:#}{A}.{A:#}{A}bar{A:#}{A}.{A:#}{A}baz{A:#}"),
        ),
        ((false, &["foo bar"], false), format!("{Q}\"foo bar\"{Q:#}")),
        (
            (true, &["foo bar"], false),
            format!("{A}.{A:#}{Q}\"foo bar\"{Q:#}"),
        ),
        (
            (false, &["foo.bar", "baz", "qu\"ux"], false),
            format!("{Q}\"foo.bar\"{Q:#}{A}.{A:#}{A}baz{A:#}{A}.{A:#}{Q}\"qu\"ux\"{Q:#}"),
        ),
        // NixOS
        (
            (false, &[], true),
            format!("{Q}(empty){Q:#} {N}(NixOS){N:#}"),
        ),
        ((true, &[], true), format!("{A}.{A:#} {N}(NixOS){N:#}")),
        (
            (false, &["foo"], true),
            format!("{A}foo{A:#} {N}(NixOS){N:#}"),
        ),
        (
            (true, &["foo"], true),
            format!("{A}.{A:#}{A}foo{A:#} {N}(NixOS){N:#}"),
        ),
    ];

    for (input, expected) in cases {
        let input = mk(input.0, input.1, input.2);
        let output = input.display().to_string();
        assert_eq!(
            expected, output,
            "{input:?} unexpected result of AttrPath::display",
        );
    }
}
