use std::io::Write;

use anstream::StripStream;
use unicode_width::UnicodeWidthStr;

use super::{AttrPath, ParseError};

fn mk<const N: usize>(leading_dot: bool, parts: [&str; N]) -> AttrPath {
    AttrPath {
        leading_dot,
        parts: parts.into_iter().map(Into::into).collect(),
    }
}

#[test]
fn cli_arg_roundtrip() -> eyre::Result<()> {
    let cases = [
        // simple
        ("a", mk(false, ["a"]), "a"),
        (
            "foo.bar.baz",
            mk(false, ["foo", "bar", "baz"]),
            "foo.bar.baz",
        ),
        // leading dot
        (".", mk(true, []), "."),
        (".foo", mk(true, ["foo"]), ".foo"),
        (".foo.bar", mk(true, ["foo", "bar"]), ".foo.bar"),
        // quoting
        ("\"foo\"", mk(false, ["foo"]), "foo"),
        ("f\"o\"o", mk(false, ["foo"]), "foo"),
        ("\".foo\"", mk(false, [".foo"]), "\".foo\""),
        (
            "\"foo.bar.baz\"",
            mk(false, ["foo.bar.baz"]),
            "\"foo.bar.baz\"",
        ),
        (
            "foo.\"bar.baz\".quux",
            mk(false, ["foo", "bar.baz", "quux"]),
            "foo.\"bar.baz\".quux",
        ),
        (
            "foo.\"bar.baz\".quux.\"\".more\".\"dots",
            mk(false, ["foo", "bar.baz", "quux", "", "more.dots"]),
            "foo.\"bar.baz\".quux.\"\".\"more.dots\"",
        ),
        // empty
        ("", mk(false, []), ""),
        ("\"\"", mk(false, [""]), "\"\""),
        (
            ".\"\".foo.\"\".bar",
            mk(true, ["", "foo", "", "bar"]),
            ".\"\".foo.\"\".bar",
        ),
    ];

    for case in cases {
        let parsed = AttrPath::parse_cli_arg(case.0)?;
        assert_eq!(
            parsed,
            case.1,
            "{:?}: unexpected result of {}",
            case.0,
            stringify!(AttrPath::parse_cli_arg),
        );

        let unparsed = parsed.to_cli_arg()?.to_string();
        assert_eq!(
            unparsed,
            case.2,
            "{:?}: unexpected result of {}",
            case.0,
            stringify!(AttrPath::to_cli_arg),
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
            "{:?}: unexpected result of {}",
            case.0,
            stringify!(AttrPath::parse_cli_arg),
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
            let case = AttrPath {
                leading_dot,
                parts: parts.iter().copied().map(Into::into).collect(),
            };

            let expected_width = case.display_width();

            let mut stream = StripStream::new(Vec::new());
            write!(stream, "{}", case.display())?;
            let output = String::from_utf8(stream.into_inner())?;
            let actual_width = UnicodeWidthStr::width(output.as_str());

            assert_eq!(
                expected_width,
                actual_width,
                "{:?}: {} output doesn't match the actual width of {}",
                case,
                stringify!(AttrPath::display_width),
                stringify!(AttrPath::display),
            );
        }
    }
    Ok(())
}

#[test]
fn display() {
    use crate::styles::{ATTR_PATH as A, ATTR_PATH_QUOTED as Q};
    let cases = [
        (mk(false, []), format!("{Q}(empty){Q:#}")),
        (mk(true, []), format!("{A}.{A:#}")),
        (mk(false, ["foo"]), format!("{A}foo{A:#}")),
        (mk(true, ["foo"]), format!("{A}.{A:#}{A}foo{A:#}")),
        (
            mk(false, ["foo", "bar", "baz"]),
            format!("{A}foo{A:#}{A}.{A:#}{A}bar{A:#}{A}.{A:#}{A}baz{A:#}"),
        ),
        (
            mk(true, ["foo", "bar", "baz"]),
            format!("{A}.{A:#}{A}foo{A:#}{A}.{A:#}{A}bar{A:#}{A}.{A:#}{A}baz{A:#}"),
        ),
        (mk(false, ["foo bar"]), format!("{Q}\"foo bar\"{Q:#}")),
        (
            mk(true, ["foo bar"]),
            format!("{A}.{A:#}{Q}\"foo bar\"{Q:#}"),
        ),
        (
            mk(false, ["foo.bar", "baz", "qu\"ux"]),
            format!("{Q}\"foo.bar\"{Q:#}{A}.{A:#}{A}baz{A:#}{A}.{A:#}{Q}\"qu\"ux\"{Q:#}"),
        ),
    ];

    for case in cases {
        let output = case.0.display().to_string();
        assert_eq!(
            output,
            case.1,
            "{:?} unexpected result of {}",
            case.0,
            stringify!(AttrPath::display),
        );
    }
}
